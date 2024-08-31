FROM python:3.12

WORKDIR ./

COPY requirements.txt ./

COPY docker-entrypoint.sh ./

RUN python3 -m venv .venv

RUN . .venv/bin/activate && pip install -r ./requirements.txt

COPY . ./

RUN chmod +x ./docker-entrypoint.sh

ENTRYPOINT ["./docker-entrypoint.sh"]

CMD [".venv/bin/python3", "-u", "main.py"]
