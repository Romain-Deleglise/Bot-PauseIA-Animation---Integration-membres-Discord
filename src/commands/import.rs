//! `/forum importer` et `/forum republier`.
//!
//! Le CDC décrivait 38 messages et 40 rôles. Les saisir un par un en commande
//! slash prendrait des heures et multiplierait les fautes de frappe : le
//! contenu est donc rédigé dans un fichier TOML versionné, relu comme un texte,
//! puis importé en le glissant dans la commande.
//!
//! L'import est **rejouable** et ne supprime jamais rien : il crée ce qui
//! manque, met à jour ce qui a changé, et signale ce qui ne figure plus dans le
//! fichier. Une suppression silencieuse reprendrait des rôles à des membres.
//!
//! Passé l'amorçage, c'est Discord qui fait foi : le CDC demande de pouvoir
//! éditer un message en quelques secondes depuis le serveur.

use crate::commands::{self, threads::autocomplete_thread};
use crate::db;
use crate::db::categories::Category;
use crate::db::posts::Post;
use crate::discord::{channel, embed};
use crate::ids;
use crate::mentions;
use crate::state::{Context, Error};
use poise::serenity_prelude as serenity;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};

/// Taille maximale acceptée pour le fichier de contenu.
const MAX_FILE_BYTES: u32 = 512 * 1024;

#[derive(Debug, Deserialize)]
pub struct Content {
    #[serde(default)]
    pub fils: Vec<ThreadSpec>,
}

#[derive(Debug, Deserialize)]
pub struct ThreadSpec {
    /// Absent pour un fil créé à la main : son nom sur Discord est repris.
    #[serde(default)]
    pub nom: Option<String>,
    /// Identifiant du fil de forum, en chaîne : un snowflake dépasse ce que
    /// TOML garantit pour un entier.
    pub salon: String,
    pub couleur: Option<String>,
    pub illustration: Option<String>,
    pub description: Option<String>,
    /// Nom du rôle parent, accordé en plus du sien par chaque message du fil.
    pub role_parent: Option<String>,
    pub mp: Option<String>,
    /// Confirmer en privé chaque entrée et chaque sortie de ce fil.
    #[serde(default)]
    pub confirmations: bool,
    /// Salon où annoncer les mains levées, en chaîne comme tout snowflake.
    pub salon_notifications: Option<String>,
    #[serde(default)]
    pub messages: Vec<PostSpec>,
}

impl ThreadSpec {
    /// Nom écrit dans le fichier, s'il y en a un.
    pub fn given_name(&self) -> Option<&str> {
        self.nom
            .as_deref()
            .map(str::trim)
            .filter(|name| !name.is_empty())
    }

    /// Désignation du fil dans les messages d'erreur, avant d'avoir interrogé
    /// Discord : son nom, à défaut son salon.
    pub fn label(&self) -> &str {
        self.given_name().unwrap_or(self.salon.trim())
    }
}

#[derive(Debug, Deserialize)]
pub struct PostSpec {
    pub slug: String,
    pub titre: String,
    /// Nom du rôle nominatif accordé. Absent : le message accorde le rôle parent
    /// du fil s'il y en a un ; sinon il n'accorde rien.
    pub role: Option<String>,
    /// Couleur propre au message, qui prime sur celle du fil.
    pub couleur: Option<String>,
    #[serde(default)]
    pub texte: String,
    /// Message d'information : aucune réaction, n'accorde rien, même dans un fil
    /// à rôle parent. Prime sur `role`.
    #[serde(default)]
    pub information: bool,
    /// Message privé propre à cette carte, qui prime sur celui du fil.
    pub mp: Option<String>,
    /// Renonce au rôle parent du fil. La carte porte toujours une main levée,
    /// mais n'accorde rien : réservé au message qui dépareille dans son fil.
    #[serde(default)]
    pub sans_role_parent: bool,
    /// `@pseudo` de la personne à prévenir quand une main se lève ici.
    pub referent: Option<String>,
    /// Salon propre à cette carte, où annoncer ses mains levées. Il l'emporte
    /// sur celui du fil : chaque projet a le sien.
    pub salon_notifications: Option<String>,
}

/// Lit le fichier et vérifie tout ce qui peut l'être sans toucher à Discord.
///
/// Tout est validé avant la moindre écriture : un import à moitié appliqué
/// laisserait le forum dans un état que personne ne saurait décrire.
pub fn parse(text: &str) -> anyhow::Result<Content> {
    let content: Content = toml::from_str(text)?;

    if content.fils.is_empty() {
        anyhow::bail!("le fichier ne décrit aucun fil");
    }

    let mut seen_threads = HashSet::new();
    let mut seen_channels = HashSet::new();
    let mut seen_roles: HashMap<String, String> = HashMap::new();

    for thread in &content.fils {
        let label = thread.label();
        let channel = commands::parse_snowflake(&thread.salon)
            .map_err(|err| anyhow::anyhow!("fil `{label}` : salon, {err}"))?;
        // Le salon est la seule clé toujours présente : un fil sans nom se
        // retrouve par lui.
        if !seen_channels.insert(channel) {
            anyhow::bail!("le salon `{}` est décrit deux fois", thread.salon.trim());
        }
        if let Some(name) = thread.given_name()
            && !seen_threads.insert(db::categories::name_key(name))
        {
            anyhow::bail!("le fil `{name}` est décrit deux fois");
        }
        if let Some(colour) = &thread.couleur {
            commands::parse_colour(colour)
                .map_err(|err| anyhow::anyhow!("fil `{label}` : {err}"))?;
        }
        if let Some(url) = &thread.illustration {
            commands::parse_url(url)
                .map_err(|err| anyhow::anyhow!("fil `{label}` : illustration, {err}"))?;
        }

        // Message privé et introduction partent tels quels, sans embed : au-delà
        // de la limite, Discord refuse l'envoi et le fil reste sans introduction.
        if let Some(excess) = thread
            .mp
            .as_deref()
            .and_then(commands::too_long_for_a_message)
        {
            anyhow::bail!("fil `{label}` : le message privé fait {excess}");
        }
        if let Some(excess) = thread
            .description
            .as_deref()
            .and_then(commands::too_long_for_a_message)
        {
            anyhow::bail!("fil `{label}` : l'introduction fait {excess}");
        }

        let mut seen_slugs = HashSet::new();
        for post in &thread.messages {
            if post.slug.trim().is_empty() {
                anyhow::bail!(
                    "fil `{label}` : le message `{}` n'a pas de slug",
                    post.titre
                );
            }
            if post.titre.trim().is_empty() {
                anyhow::bail!(
                    "fil `{label}` : le message `{}` n'a pas de titre",
                    post.slug
                );
            }
            // Au-delà, la carte est tronquée à la publication, sans un mot :
            // la coupe se découvre en lisant le forum.
            if post.titre.chars().count() > embed::TITLE_LIMIT {
                anyhow::bail!(
                    "fil `{label}` : le titre du message `{}` fait {} caractères, maximum {}",
                    post.slug,
                    post.titre.chars().count(),
                    embed::TITLE_LIMIT
                );
            }
            if post.texte.chars().count() > embed::DESCRIPTION_LIMIT {
                anyhow::bail!(
                    "fil `{label}` : le texte du message `{}` fait {} caractères, maximum {}",
                    post.slug,
                    post.texte.chars().count(),
                    embed::DESCRIPTION_LIMIT
                );
            }
            if let Some(colour) = &post.couleur {
                commands::parse_colour(colour)
                    .map_err(|err| anyhow::anyhow!("message `{}` : {err}", post.slug))?;
            }
            if !seen_slugs.insert(post.slug.trim().to_owned()) {
                anyhow::bail!("fil `{label}` : le slug `{}` apparaît deux fois", post.slug);
            }
            // Un rôle ne peut appartenir qu'à un message : autant s'en
            // apercevoir en relisant le fichier plutôt qu'au dernier insert.
            if let Some(role) = &post.role
                && let Some(other) = seen_roles.insert(role.to_lowercase(), post.slug.clone())
            {
                anyhow::bail!(
                    "le rôle `{role}` est réclamé par `{other}` et par `{}`",
                    post.slug
                );
            }
        }
    }

    Ok(content)
}

/// Importer le contenu du forum depuis un fichier TOML.
#[poise::command(slash_command)]
pub async fn importer(
    ctx: Context<'_>,
    #[description = "Fichier de contenu au format TOML"] fichier: serenity::Attachment,
) -> Result<(), Error> {
    commands::begin(ctx).await?;

    if fichier.size > MAX_FILE_BYTES {
        ctx.say(format!(
            "Fichier trop volumineux ({} octets, maximum {MAX_FILE_BYTES}).",
            fichier.size
        ))
        .await?;
        return Ok(());
    }

    let bytes = fichier.download().await?;
    let text = match String::from_utf8(bytes) {
        Ok(text) => text,
        Err(_) => {
            ctx.say("Le fichier n'est pas encodé en UTF-8.").await?;
            return Ok(());
        }
    };
    let content = match parse(&text) {
        Ok(content) => content,
        Err(err) => {
            ctx.say(format!("Fichier invalide : {err}")).await?;
            return Ok(());
        }
    };

    // Vérifier tous les salons avant la moindre écriture : un import
    // interrompu au troisième fil laisserait le forum à moitié construit, et
    // les identifiants du fichier livré sont des valeurs à remplacer.
    let mut unreachable = Vec::new();
    // Nom actuel de chaque fil sur Discord, repris quand le fichier n'en donne
    // pas : un fil créé à la main a déjà le sien.
    let mut discord_names: HashMap<String, String> = HashMap::new();
    for thread in &content.fils {
        let channel = serenity::ChannelId::new(commands::parse_snowflake(&thread.salon)?);
        match channel.to_channel(ctx).await {
            Ok(serenity::Channel::Guild(guild_channel))
                if commands::is_writable(guild_channel.kind) =>
            {
                discord_names.insert(thread.salon.trim().to_owned(), guild_channel.name);
            }
            Ok(_) => unreachable.push(format!(
                "**{}** : <#{channel}> ne peut pas recevoir de messages",
                thread.label()
            )),
            Err(err) => unreachable.push(format!(
                "**{}** : salon `{}` introuvable ({err})",
                thread.label(),
                thread.salon
            )),
        }
    }
    if !unreachable.is_empty() {
        ctx.say(format!(
            "Import annulé, aucun changement n'a été fait :\n{}",
            unreachable
                .iter()
                .map(|line| format!("- {line}"))
                .collect::<Vec<_>>()
                .join("\n")
        ))
        .await?;
        return Ok(());
    }

    let mut report = Report::default();
    let mut roles = guild_roles_by_name(ctx).await?;
    let members = commands::member_handles(ctx).await?;

    for thread in &content.fils {
        let name = thread
            .given_name()
            .map(str::to_owned)
            .or_else(|| discord_names.get(thread.salon.trim()).cloned())
            .unwrap_or_else(|| thread.salon.trim().to_owned());
        let category = upsert_thread(ctx, thread, &name, &mut roles, &mut report).await?;
        let known: HashSet<String> = thread
            .messages
            .iter()
            .map(|post| post.slug.trim().to_owned())
            .collect();

        for (rank, post) in thread.messages.iter().enumerate() {
            upsert_post(
                ctx,
                &category,
                post,
                rank as i64 + 1,
                &members,
                &mut roles,
                &mut report,
            )
            .await?;
        }

        for orphan in db::posts::by_category(&ctx.data().db, category.id).await? {
            if !known.contains(&orphan.slug) {
                report
                    .orphans
                    .push(format!("{} › {}", category.name, orphan.title));
            }
        }
    }
    ctx.data().reload_caches().await?;

    commands::say_long(ctx, report.render()).await?;
    Ok(())
}

/// Exporter le contenu actuel du forum au format du fichier.
///
/// C'est ce qui rend l'import sans danger : on exporte ce que le bot a
/// réellement en base, on le compare à ce qu'on voulait changer, on fusionne,
/// et on réimporte. Sans cela, l'import écrase en silence tout ce qui a été
/// corrigé depuis Discord.
#[poise::command(slash_command, rename = "exporter")]
pub async fn exporter(ctx: Context<'_>) -> Result<(), Error> {
    commands::begin(ctx).await?;

    let Some(guild_id) = ctx.guild_id() else {
        ctx.say("Commande réservée à un serveur.").await?;
        return Ok(());
    };
    // Les rôles et les référents sont stockés par identifiant ; le fichier, lui,
    // se lit et s'écrit avec des noms.
    let roles = guild_id.roles(ctx.http()).await?;
    let name_of = |role_id: i64| -> Option<String> {
        roles
            .get(&ids::role(role_id))
            .map(|role| role.name.to_string())
    };

    let mut out = String::from(
        "# Exporté depuis Discord par /forum exporter.\n\
         # Comparez ce fichier au vôtre avant de réimporter : il contient tout\n\
         # ce qui a été modifié depuis Discord, et que l'import écraserait.\n",
    );
    for category in db::categories::list(&ctx.data().db).await? {
        out.push_str("\n[[fils]]\n");
        out.push_str(&field("nom", &category.name));
        out.push_str(&field("salon", &category.channel_id.to_string()));
        if let Some(colour) = category.colour {
            out.push_str(&field("couleur", &format!("#{colour:06X}")));
        }
        if let Some(url) = &category.header_image_url {
            out.push_str(&field("illustration", url));
        }
        if let Some(parent) = category.parent_role_id.and_then(name_of) {
            out.push_str(&field("role_parent", &parent));
        }
        if let Some(channel) = category.notify_channel_id {
            out.push_str(&field("salon_notifications", &channel.to_string()));
        }
        if category.confirmations {
            out.push_str("confirmations = true\n");
        }
        if let Some(text) = filled(&category.dm_text) {
            out.push_str(&long_field("mp", text));
        }

        for post in db::posts::by_category(&ctx.data().db, category.id).await? {
            out.push_str("\n[[fils.messages]]\n");
            out.push_str(&field("slug", &post.slug));
            out.push_str(&field("titre", &post.title));
            if let Some(role) = post.role_id.and_then(name_of) {
                out.push_str(&field("role", &role));
            }
            if let Some(colour) = post.colour {
                out.push_str(&field("couleur", &format!("#{colour:06X}")));
            }
            if post.information {
                out.push_str("information = true\n");
            }
            if !post.grants_parent {
                out.push_str("sans_role_parent = true\n");
            }
            if let Some(referent) = post.referent_id {
                // Une mention est relue telle quelle par l'import, là où un
                // pseudo devrait être retrouvé dans la liste des membres.
                out.push_str(&field("referent", &format!("<@{referent}>")));
            }
            if let Some(channel) = post.notify_channel_id {
                out.push_str(&field("salon_notifications", &channel.to_string()));
            }
            if let Some(text) = filled(&post.dm_text) {
                out.push_str(&long_field("mp", text));
            }
            if !post.body.trim().is_empty() {
                out.push_str(&long_field("texte", &post.body));
            }
        }
    }

    ctx.send(
        poise::CreateReply::default()
            .attachment(serenity::CreateAttachment::bytes(
                out.into_bytes(),
                "forum.toml",
            ))
            .content(
                "Voici le contenu actuel du bot. Comparez-le au vôtre avant de réimporter : \
                 tout ce qui a été modifié depuis Discord s'y trouve.",
            )
            .ephemeral(true),
    )
    .await?;
    Ok(())
}

/// Le pseudo d'un référent, débarrassé de ce qui n'en fait pas partie : l'arobase
/// qui le précède, et la ponctuation de la phrase dont il a été recopié.
fn clean_handle(raw: &str) -> &str {
    raw.trim()
        .trim_start_matches('@')
        .trim_end_matches(['.', ',', ';', ':'])
}

/// Une valeur sur une ligne, guillemets et antislashs échappés.
fn field(key: &str, value: &str) -> String {
    let escaped = value.replace('\\', "\\\\").replace('"', "\\\"");
    format!("{key} = \"{escaped}\"\n")
}

/// Une valeur multiligne. Le premier saut de ligne est avalé par TOML, d'où le
/// texte qui commence à la ligne suivante.
fn long_field(key: &str, value: &str) -> String {
    let escaped = value
        .replace('\\', "\\\\")
        .replace("\"\"\"", "\\\"\\\"\\\"");
    format!("{key} = \"\"\"\n{escaped}\"\"\"\n")
}

fn filled(text: &Option<String>) -> Option<&str> {
    text.as_deref().filter(|value| !value.trim().is_empty())
}

/// Réécrire les fils pour rétablir l'ordre d'affichage.
///
/// Republier efface et repose chaque carte : les réactions des membres
/// disparaissent avec les anciens messages. Les rôles déjà accordés restent,
/// mais plus rien ne les justifie ensuite, et un membre qui retire une réaction
/// reposée ne perdra pas un rôle qu'aucune ligne ne relie plus à lui. D'où la
/// confirmation, comme pour `/forum fil supprimer`.
#[poise::command(slash_command)]
pub async fn republier(
    ctx: Context<'_>,
    #[description = "Fil à republier. Défaut : tous"]
    #[autocomplete = "autocomplete_thread"]
    fil: Option<String>,
    #[description = "Confirmer : les réactions des membres seront effacées"] confirmer: Option<
        bool,
    >,
) -> Result<(), Error> {
    commands::begin(ctx).await?;

    let categories = match fil.as_deref() {
        None => db::categories::list(&ctx.data().db).await?,
        Some(name) => match db::categories::by_name(&ctx.data().db, name).await? {
            Some(category) => vec![category],
            None => {
                ctx.say(format!("Le fil **{name}** n'existe pas.")).await?;
                return Ok(());
            }
        },
    };
    if categories.is_empty() {
        ctx.say("Aucun fil à republier.").await?;
        return Ok(());
    }

    if confirmer != Some(true) {
        let mut posts = 0;
        for category in &categories {
            posts += db::posts::by_category(&ctx.data().db, category.id)
                .await?
                .len();
        }
        let names = categories
            .iter()
            .map(|category| format!("**{}**", category.name))
            .collect::<Vec<_>>()
            .join(", ");
        ctx.say(format!(
            "Republier {names} effacera puis reposera {posts} carte·s, et avec elles **toutes les mains levées** de ce·s fil·s. Les membres gardent les rôles déjà reçus, mais devront lever la main à nouveau pour que le bot en tienne compte.\n\nÀ n'utiliser que pour réordonner ou pour afficher une nouvelle introduction. Pour corriger un texte, `/forum message éditer` garde les réactions.\n\nRelancez avec `confirmer: True`."
        ))
        .await?;
        return Ok(());
    }

    let mut total = 0;
    for category in &categories {
        total += channel::rewrite(ctx.http(), ctx.data(), category).await?;
    }
    ctx.data().reload_caches().await?;

    ctx.say(format!(
        "{} fil·s republié·s, {total} message·s.\nLes réactions ont été remises à zéro : les membres gardent leurs rôles mais devront lever la main de nouveau pour s'en retirer.",
        categories.len()
    ))
    .await?;
    Ok(())
}

#[derive(Default)]
struct Report {
    threads_created: usize,
    threads_updated: usize,
    posts_created: usize,
    posts_updated: usize,
    roles_created: Vec<String>,
    orphans: Vec<String>,
    unknown_handles: Vec<String>,
}

impl Report {
    fn render(&self) -> String {
        let mut lines = vec![format!(
            "Import terminé : {} fil·s créé·s, {} mis à jour, {} message·s publié·s, {} mis à jour.",
            self.threads_created, self.threads_updated, self.posts_created, self.posts_updated
        )];
        if !self.roles_created.is_empty() {
            lines.push(format!(
                "{} rôle·s créé·s : {}",
                self.roles_created.len(),
                self.roles_created.join(", ")
            ));
        }
        if !self.orphans.is_empty() {
            lines.push(format!(
                "{} message·s en base ne figurent plus dans le fichier ; ils n'ont **pas** été supprimés, car cela reprendrait des rôles à des membres. Retirez-les avec `/forum message supprimer` :\n{}",
                self.orphans.len(),
                self.orphans
                    .iter()
                    .map(|orphan| format!("- {orphan}"))
                    .collect::<Vec<_>>()
                    .join("\n")
            ));
        }
        if !self.unknown_handles.is_empty() {
            let handles: Vec<String> = self
                .unknown_handles
                .iter()
                .map(|handle| format!("@{handle}"))
                .collect();
            lines.push(format!(
                "{} pseudo·s introuvable·s sur le serveur, laissé·s en texte : {}",
                handles.len(),
                handles.join(", ")
            ));
        }
        if self.posts_created > 0 && self.posts_updated > 0 {
            lines.push(
                "Les nouveaux messages ont été publiés à la suite. Lancez `/forum republier` pour rétablir l'ordre du fichier.".to_owned(),
            );
        }
        lines.join("\n")
    }
}

/// Rôles du serveur, indexés par nom replié.
///
/// Passe par l'API plutôt que par le cache : au premier import, le bot vient
/// peut-être de démarrer.
async fn guild_roles_by_name(ctx: Context<'_>) -> Result<HashMap<String, i64>, Error> {
    Ok(ctx
        .data()
        .config
        .guild_id
        .roles(ctx.http())
        .await?
        .into_values()
        .map(|role| (role.name.to_lowercase(), ids::to_db(role.id.get())))
        .collect())
}

/// Retrouve un rôle par son nom, ou le crée.
async fn role_by_name(
    ctx: Context<'_>,
    name: &str,
    roles: &mut HashMap<String, i64>,
    report: &mut Report,
) -> Result<i64, Error> {
    let key = name.trim().trim_start_matches('@').to_lowercase();
    if let Some(role_id) = roles.get(&key) {
        return Ok(*role_id);
    }

    let created = ctx
        .data()
        .config
        .guild_id
        .create_role(
            ctx.http(),
            serenity::EditRole::new().name(name.trim().trim_start_matches('@')),
        )
        .await?;
    let role_id = ids::to_db(created.id.get());
    roles.insert(key, role_id);
    report.roles_created.push(created.name.to_string());
    Ok(role_id)
}

async fn upsert_thread(
    ctx: Context<'_>,
    spec: &ThreadSpec,
    name: &str,
    roles: &mut HashMap<String, i64>,
    report: &mut Report,
) -> Result<Category, Error> {
    let parent = match &spec.role_parent {
        Some(name) => Some(role_by_name(ctx, name, roles, report).await?),
        None => None,
    };
    let wanted = Category {
        id: 0,
        name: name.to_owned(),
        channel_id: ids::to_db(commands::parse_snowflake(&spec.salon)?),
        header_image_url: spec.illustration.clone(),
        header_text: spec.description.clone(),
        colour: spec
            .couleur
            .as_deref()
            .map(commands::parse_colour)
            .transpose()?,
        parent_role_id: parent,
        dm_text: spec.mp.clone(),
        confirmations: spec.confirmations,
        notify_channel_id: spec
            .salon_notifications
            .as_deref()
            .map(commands::parse_snowflake)
            .transpose()?
            .map(ids::to_db),
        // Le fichier de contenu ne décrit pas les confirmations : elles se
        // règlent depuis Discord, et un import ne doit pas les écraser.
        joined_text: None,
        left_text: None,
    };

    // Le salon d'abord : le nom repris de Discord peut avoir changé depuis le
    // dernier import.
    let existing = match db::categories::by_channel(&ctx.data().db, wanted.channel_id).await? {
        Some(found) => Some(found),
        None => db::categories::by_name(&ctx.data().db, &wanted.name).await?,
    };
    match existing {
        Some(existing) => {
            db::categories::update(
                &ctx.data().db,
                &Category {
                    id: existing.id,
                    joined_text: existing.joined_text.clone(),
                    left_text: existing.left_text.clone(),
                    ..wanted
                },
            )
            .await?;
            report.threads_updated += 1;
            Ok(db::categories::by_id(&ctx.data().db, existing.id)
                .await?
                .expect("mis à jour à l'instant"))
        }
        None => {
            let created = db::categories::insert(&ctx.data().db, &wanted).await?;
            report.threads_created += 1;
            Ok(created)
        }
    }
}

async fn upsert_post(
    ctx: Context<'_>,
    category: &Category,
    spec: &PostSpec,
    position: i64,
    members: &HashMap<String, u64>,
    roles: &mut HashMap<String, i64>,
    report: &mut Report,
) -> Result<(), Error> {
    // Un message d'information n'accorde rien : on ne crée même pas de rôle.
    let role_id = if spec.information {
        None
    } else {
        match &spec.role {
            Some(name) => Some(role_by_name(ctx, name, roles, report).await?),
            None => None,
        }
    };
    let (body, unknown) = mentions::link_handles(spec.texte.trim(), members);
    for handle in unknown {
        if !report.unknown_handles.contains(&handle) {
            report.unknown_handles.push(handle);
        }
    }
    // Le référent est noté `@pseudo` comme dans les textes : il se résout sur la
    // liste des membres, et son absence se signale plutôt que d'échouer.
    let referent = match spec.referent.as_deref().map(clean_handle) {
        // `<@123>` est ce qu'écrit `/forum exporter` : l'identifiant s'y lit
        // directement, sans dépendre d'un pseudo qui peut avoir changé.
        Some(handle) if handle.starts_with("<@") => handle
            .trim_start_matches("<@")
            .trim_end_matches('>')
            .parse::<u64>()
            .ok()
            .map(ids::to_db),
        Some(handle) => match members.get(&handle.to_lowercase()) {
            Some(id) => Some(ids::to_db(*id)),
            None => {
                let handle = handle.to_owned();
                if !report.unknown_handles.contains(&handle) {
                    report.unknown_handles.push(handle);
                }
                None
            }
        },
        None => None,
    };

    let existing = db::posts::by_slug(&ctx.data().db, category.id, spec.slug.trim()).await?;
    let post = Post {
        id: existing.as_ref().map(|post| post.id).unwrap_or(0),
        category_id: category.id,
        slug: spec.slug.trim().to_owned(),
        title: spec.titre.trim().to_owned(),
        body,
        role_id,
        colour: spec
            .couleur
            .as_deref()
            .map(commands::parse_colour)
            .transpose()?,
        message_id: existing.as_ref().and_then(|post| post.message_id),
        position,
        information: spec.information,
        dm_text: spec.mp.clone(),
        grants_parent: !spec.sans_role_parent,
        notify_channel_id: spec
            .salon_notifications
            .as_deref()
            .map(commands::parse_snowflake)
            .transpose()?
            .map(ids::to_db),
        referent_id: referent,
    };

    match existing {
        Some(_) => {
            db::posts::update(&ctx.data().db, &post).await?;
            // Édition sur place : les réactions et les rôles déjà accordés
            // survivent au réimport.
            channel::refresh(ctx.http(), ctx.data(), category, &post).await?;
            report.posts_updated += 1;
        }
        None => {
            let stored = db::posts::insert(&ctx.data().db, &post).await?;
            channel::publish(ctx.http(), ctx.data(), category, &stored).await?;
            report.posts_created += 1;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const MINIMAL: &str = r#"
[[fils]]
nom = "Groupes locaux"
salon = "1234567890123456789"

[[fils.messages]]
slug = "paris"
titre = "Paris"
role = "gl-paris"
texte = "Un groupe très actif."
"#;

    #[test]
    fn a_minimal_file_parses() {
        let content = parse(MINIMAL).unwrap();
        assert_eq!(content.fils.len(), 1);
        assert_eq!(content.fils[0].messages.len(), 1);
        assert_eq!(
            content.fils[0].messages[0].role.as_deref(),
            Some("gl-paris")
        );
    }

    #[test]
    fn an_empty_file_is_refused() {
        assert!(parse("").is_err());
        assert!(parse("fils = []").is_err());
    }

    #[test]
    fn a_thread_needs_a_valid_channel() {
        let broken = MINIMAL.replace("1234567890123456789", "pas-un-identifiant");
        assert!(parse(&broken).is_err());
    }

    #[test]
    fn duplicate_threads_are_refused() {
        let doubled = format!("{MINIMAL}\n[[fils]]\nnom = \"GROUPES LOCAUX\"\nsalon = \"99\"\n");
        // Les deux noms se replient sur la même clé : l'autocomplétion et la
        // base ne sauraient les distinguer.
        assert!(parse(&doubled).is_err());
    }

    #[test]
    fn a_thread_may_leave_its_name_to_discord() {
        let unnamed = MINIMAL.replace("nom = \"Groupes locaux\"\n", "");
        let content = parse(&unnamed).unwrap();
        assert_eq!(content.fils[0].given_name(), None);
        assert_eq!(content.fils[0].label(), "1234567890123456789");
    }

    #[test]
    fn a_channel_described_twice_is_refused() {
        let doubled = format!("{MINIMAL}\n[[fils]]\nsalon = \"1234567890123456789\"\n");
        assert!(parse(&doubled).is_err());
    }

    #[test]
    fn duplicate_slugs_within_a_thread_are_refused() {
        let doubled =
            format!("{MINIMAL}\n[[fils.messages]]\nslug = \"paris\"\ntitre = \"Paris bis\"\n");
        assert!(parse(&doubled).is_err());
    }

    #[test]
    fn a_role_claimed_twice_is_refused() {
        let doubled = format!(
            "{MINIMAL}\n[[fils.messages]]\nslug = \"paris-2\"\ntitre = \"Paris bis\"\nrole = \"GL-Paris\"\n"
        );
        // La contrainte existe en base ; l'attraper à la lecture évite un import
        // interrompu au milieu.
        assert!(parse(&doubled).is_err());
    }

    #[test]
    fn a_message_colour_is_validated() {
        assert!(parse(&format!("{MINIMAL}couleur = \"#99AAB5\"\n")).is_ok());
        assert!(parse(&format!("{MINIMAL}couleur = \"gris\"\n")).is_err());
    }

    #[test]
    fn a_message_without_role_is_accepted() {
        let info = r#"
[[fils]]
nom = "Projets"
salon = "1"

[[fils.messages]]
slug = "intro"
titre = "Bienvenue"
texte = "Levez la main pour rejoindre un projet."
"#;
        let content = parse(info).unwrap();
        assert!(content.fils[0].messages[0].role.is_none());
    }

    /// Le fichier réellement livré doit rester valide.
    #[test]
    fn the_shipped_content_file_is_valid() {
        let text = include_str!("../../contenu/forum.toml");
        let content = parse(text).expect("contenu/forum.toml doit être valide");

        assert_eq!(content.fils.len(), 4, "le CDC décrit quatre fils");
        let messages: usize = content.fils.iter().map(|fil| fil.messages.len()).sum();
        assert_eq!(messages, 45, "le contenu du forum décrit 45 messages");

        // Le MP est la seule explication du parcours que reçoit un membre qui
        // lève la main : aucun fil ne doit rester muet. La description tient la
        // même explication sur le fil, pour qui refuse les messages privés.
        for fil in &content.fils {
            assert!(
                fil.mp.as_deref().is_some_and(|mp| !mp.trim().is_empty()),
                "le fil `{}` n'a pas de message privé d'accueil",
                fil.label()
            );

            for message in &fil.messages {
                assert!(
                    message.titre.chars().count() <= embed::TITLE_LIMIT,
                    "le titre de `{}` serait tronqué à la publication",
                    message.slug
                );
            }
        }
    }

    /// Un fil minimal, avec un champ d'en-tête à éprouver.
    fn thread_with(field: &str, value: &str) -> String {
        format!("[[fils]]\nnom = \"Projets\"\nsalon = \"1\"\n{field} = \"\"\"{value}\"\"\"\n")
    }

    #[test]
    fn an_exported_field_can_be_read_back() {
        // Ce que `field` et `long_field` écrivent doit se relire tel quel,
        // sinon l'aller-retour perdrait un guillemet ou un antislash.
        let texte = format!(
            "[[fils]]\n{}{}{}",
            field("nom", "Projets \"spéciaux\""),
            field("salon", "123"),
            long_field("mp", "Une ligne\nUne autre avec un \\ antislash")
        );
        let lu = parse(&texte).expect("le format exporté doit être relisible");
        assert_eq!(lu.fils[0].nom.as_deref(), Some("Projets \"spéciaux\""));
        assert_eq!(
            lu.fils[0].mp.as_deref(),
            Some("Une ligne\nUne autre avec un \\ antislash")
        );
    }

    #[test]
    fn a_referent_survives_the_round_trip() {
        // L'export écrit une mention, et le nettoyage laisse intacte une
        // ponctuation recopiée depuis le texte d'une carte.
        assert_eq!(clean_handle("  @dyson4282. "), "dyson4282");
        assert_eq!(clean_handle("<@123>"), "<@123>");
    }

    #[test]
    fn a_direct_message_longer_than_discord_allows_is_refused() {
        let text = thread_with("mp", &"a".repeat(commands::MAX_MESSAGE_CHARS + 1));
        let err = parse(&text).expect_err("un message privé trop long doit être refusé");
        assert!(err.to_string().contains("message privé"), "{err}");
    }

    #[test]
    fn an_introduction_longer_than_discord_allows_is_refused() {
        let text = thread_with("description", &"a".repeat(commands::MAX_MESSAGE_CHARS + 1));
        let err = parse(&text).expect_err("une introduction trop longue doit être refusée");
        assert!(err.to_string().contains("introduction"), "{err}");
    }

    #[test]
    fn a_card_that_discord_would_truncate_is_refused() {
        let long = "a".repeat(embed::DESCRIPTION_LIMIT + 1);
        let text = format!(
            "[[fils]]\nnom = \"Projets\"\nsalon = \"1\"\n\n[[fils.messages]]\nslug = \"x\"\ntitre = \"X\"\ntexte = \"\"\"{long}\"\"\"\n"
        );
        let err = parse(&text).expect_err("un texte de carte trop long doit être refusé");
        assert!(err.to_string().contains("texte du message"), "{err}");
    }
}
