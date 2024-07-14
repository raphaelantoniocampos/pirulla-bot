#!/bin/sh

set -e

# Ativando o ambiente virtual
. .venv/bin/activate

# Avaliando o comando passado:
exec "$@"
