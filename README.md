# Pirulla Bot

This project tracks the average length of videos on the "Pirulla" YouTube channel. The script collects data from the channel, calculates the average video duration, posts updates on Twitter with relevant statistics and a plot showing the variation in average duration over time.

## 🚀 Prerequisites

Python 3.x

Required Python packages

-matplotlib

-pandas

-google-api-python-client

-python-dotenv

-tweepy

## ⚙️ Configuration

Create a data directory inside the app directory

```
cd app/
mkdir data
```

Inside the app directory, set up your python virtual environment

```
python3 -m venv .venv
```

Start your virtual environment and install the dependencies
```
source .venv/bin/activate
python3 -m pip install -r requirements.txt
```

Once you have installed the dependencies, make sure to set up your Google and Twitter/X API keys and secrets. Create a .env file with the following structure:

```
DEVELOPER_KEY="Your Google API Developer Key"
CHANNEL_ID="The Channel ID of the channel you want to analyze"
TWITTER_API_KEY="Your Twitter/X API Key"
TWITTER_API_SECRET="Your Twitter/X API Secret Key"
ACCESS_TOKEN="Your Twitter/X Secret Access Token"
ACCESS_SECRET_TOKEN="Your Twitter/X Bearer Token"
BEARER_TOKEN="Your Twitter/X Access Token"
```

## 📦 Usage

Run the script by executing the following command in your terminal:

```
python3 main.py
```

The script will generate a channel_data.csv inside the data directory you've created and then continuously monitor the YouTube channel, checking for updates. When a new video is published, it calculates the average video duration, generates a plot, and posts a tweet with relevant statistics.

## 🗒️ Notes

The script uses the Google API to interact with YouTube data and the Tweepy library for posting tweets.

Twitter/X and Google API keys are required. Ensure proper configuration in the .env file.

The channel_data.csv file stores historical data to track changes in the channel statistics over time.

The generated plot (data/pirulla_plot.png) visually represents the variation in average video duration.

## 🖇️ Contributing
If you would like to contribute to this project, please feel free to open an issue or pull request.

## ✒️ Authors
Raphael Campos - [raphaelantoniocampos](https://github.com/raphaelantoniocampos)
