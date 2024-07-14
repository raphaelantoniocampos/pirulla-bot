FROM python:3.7

WORKDIR ./

COPY requirements.txt ./

COPY channel.py ./

COPY docker-entrypoint.sh ./

RUN python3 -m venv .venv

RUN . .venv/bin/activate && pip install -r ./requirements.txt

COPY . ./

RUN chmod +x ./docker-entrypoint.sh

COPY channel.py .venv/lib/python3.7/site-packages/pytube/contrib/

ENTRYPOINT ["./docker-entrypoint.sh"]

CMD [".venv/bin/python3", "-u", "main.py"]
