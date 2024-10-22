FROM python:3.11-bookworm AS builder

ENV PYTHONUNBUFFERED=1 \ 
    PYTHONDONTWRITEBYTECODE=1 

RUN pip install poetry && poetry config virtualenvs.in-project true

WORKDIR /app

COPY pyproject.toml poetry.lock ./

RUN poetry install


FROM python:3.11-slim-bookworm

WORKDIR /app

COPY --from=builder /app .
COPY pirulla-bot/ ./pirulla-bot/

CMD ["/app/.venv/bin/python", "pirulla-bot/main.py"]

