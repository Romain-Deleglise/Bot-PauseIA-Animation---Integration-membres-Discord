#!/usr/bin/env bash
# Vérifie que les liens du contenu du forum répondent encore.
#
# Un lien mort dans un message privé ne se voit pas : le bot l'envoie, le membre
# tombe sur une page d'erreur, et personne ne l'apprend. Les pages Notion sont
# renommées, dépubliées ou déplacées sans prévenir, et le contenu du forum en
# cite une poignée qui font tout le parcours d'accueil.
#
# Usage :
#   deploy/check-links.sh [fichier…]     par défaut contenu/forum.toml
#   DRY_RUN=1 deploy/check-links.sh      liste les liens sans rien appeler
#
# Sortie : 1 si un lien est introuvable (404, 410, domaine injoignable).
# Un refus d'accès (403, 405, 429) n'échoue pas : Calendly, WhatsApp et Discord
# filtrent les requêtes automatisées, et leur refus ne dit rien du lien.

set -uo pipefail

cd "$(dirname "$0")/.."
files=("${@:-contenu/forum.toml}")

# Les parenthèses et la ponctuation finale ne font pas partie de l'URL.
urls=$(grep -ohE 'https?://[^ )"<>]+' "${files[@]}" \
    | sed -E 's/[.,;:!?)]+$//' \
    | sort -u)

if [[ -z "$urls" ]]; then
    echo "Aucun lien trouvé dans ${files[*]}." >&2
    exit 1
fi

if [[ -n "${DRY_RUN:-}" ]]; then
    echo "$urls"
    exit 0
fi

# Les sites grand public répondent différemment à un client sans navigateur.
agent='Mozilla/5.0 (compatible; bot-roles-liens/1; +https://pauseia.fr)'
broken=0
suspicious=0

while read -r url; do
    code=$(curl -sS -o /dev/null -w '%{http_code}' \
        -L --max-time 20 --retry 2 --retry-delay 2 \
        -A "$agent" "$url" 2>/dev/null) || code="000"

    case "$code" in
        2??|3??)
            printf '  ok    %s  %s\n' "$code" "$url"
            ;;
        404|410)
            printf '  MORT  %s  %s\n' "$code" "$url"
            broken=$((broken + 1))
            ;;
        *)
            # 000 = domaine injoignable ou TLS refusé, souvent un vrai problème,
            # mais aussi ce que renvoie un réseau de CI sans sortie directe.
            printf '  ?     %s  %s\n' "$code" "$url"
            suspicious=$((suspicious + 1))
            ;;
    esac
done <<< "$urls"

echo
if (( broken > 0 )); then
    echo "$broken lien·s introuvable·s : à corriger dans le contenu du forum." >&2
    exit 1
fi
if (( suspicious > 0 )); then
    echo "$suspicious lien·s sans réponse claire, à ouvrir à la main."
fi
echo "Aucun lien mort."
