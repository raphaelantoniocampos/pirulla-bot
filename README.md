# Pirulla Bot

This project tracks the average length of videos on the "Pirulla" YouTube channel. The script collects data from the channel, calculates the average video duration, posts updates on Twitter with relevant statistics and a plot showing the variation in average duration over time.

## 🚀 Prerequisites
Python 3.x

Required Python packages (install using pip install -r requirements.txt):

-os

-datetime

-time

-random

-json

-matplotlib

-pytube

-tweepy

## ⚙️ Configuration

Once you have installed the dependencies, you can clone the repository and run the following command to generate the data the bot will need:

```
python3 data_generator.py
```
It will create the channel_data.json file.

Before running the script, make sure to set up your Twitter API keys and secrets. Create a file named credentials.json with the following structure:

```
{
  "API key": "your_api_key",
  "API secret key": "your_api_secret_key",
  "Access Token": "your_access_token",
  "Access Secret Token": "your_access_secret_token",
  "Bearer Token": "your_bearer_token"
}
```

## 📦 Usage

Run the script by executing the following command in your terminal:

```
python3 main.py
```

The script will continuously monitor the YouTube channel, checking for updates. When a new video is published, it calculates the average video duration, generates a plot, and posts a tweet with relevant statistics.

## 🗒️ Notes

The script uses the PyTube library to interact with YouTube data and the Tweepy library for posting tweets.

Twitter API keys are required for posting tweets. Ensure proper configuration in the credentials.json file.

The channel_data.json file stores historical data to track changes in the channel statistics over time.

The generated plot (pirulla_plot.png) visually represents the variation in average video duration.

## 🖇️ Contributing
If you would like to contribute to this project, please feel free to open an issue or pull request.

## ✒️ Authors
Raphael Campos - [raphaelantoniocampos](https://github.com/raphaelantoniocampos)
