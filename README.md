# Pirulla Bot

This project tracks the average length of videos on the Pirulla YouTube channel. It uses the Pytube library to access YouTube data and Tweepy library to post updates to Twitter when there are changes in the average length.

## 🚀 Getting Started
To get started, you will need to install the following dependencies:

Python 3

Tweepy

Matplotlib

Pytube

Once you have installed the dependencies, you can clone the repository and run the following command to generate the data the bot will need:

```
python3 data_generator.py
```
It will create the channel_data.json file.

Create a Twitter account and enable developer mode, create a file called credentials.json and save the API keys in it.

To start the bot run the following command:

```
python3 main.py
```

## 📦 Usage
The tracker will run indefinitely, checking for updates to the Pirulla YouTube channel every 30-60 minutes. When there are changes in the average length, the tracker will post an update to Twitter.

## 🖇️ Contributing
If you would like to contribute to this project, please feel free to open an issue or pull request.

## ✒️ Authors
Raphael Campos - [raphaelantoniocampos](https://github.com/raphaelantoniocampos)
