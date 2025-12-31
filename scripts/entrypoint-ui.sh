#!/usr/bin/env bash
set -euo pipefail

envsubst '$QLEVER_PORT' < /app/Qleverfile-ui.template.yml > /app/Qleverfile-ui.yml
python manage.py config default /app/Qleverfile-ui.yml --hide-all-other-backends
exec gunicorn \
--bind :7000 \
--workers 3 \
--limit-request-line 10000 \
qlever.wsgi:application
