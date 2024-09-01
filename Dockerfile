FROM python:3.11

WORKDIR /app

COPY requirements.txt ./

COPY docker-entrypoint.sh ./

RUN python3 -m venv .venv

RUN . .venv/bin/activate && pip install --no-cache-dir -r requirements.txt

COPY ./app /app

RUN mkdir -p ./data/

RUN chmod +x ./docker-entrypoint.sh

ENTRYPOINT ["./docker-entrypoint.sh"]

CMD [".venv/bin/python3", "-u", "main.py"]
