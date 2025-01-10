import pandas as pd
import re
import logging

import googleapiclient.discovery


class YoutubeAPI:
    def __init__(self, config):
        logging.basicConfig(level=logging.INFO)
        self.logger = logging.getLogger(__name__)
        self.api_service_name = "youtube"
        self.api_version = "v3"

        self.config = config
        self.channel_id = config.CHANNEL_ID
        self.developer_key = config.DEVELOPER_KEY

        self.youtube = googleapiclient.discovery.build(
            self.api_service_name, self.api_version, developerKey=self.developer_key
        )

    def get_stored_data(self) -> pd.DataFrame:
        """Read stored channel data from CSV."""
        try:
            df = pd.read_csv(
                "data/channel_data.csv",
                sep=",",
                index_col=0,
                dtype={
                    "video_id": str,
                    "title": str,
                    "url": str,
                    "publishedAt": str,
                    "duration": int,
                    "currentMean": float,
                },
            )

            # Convert 'publishedAt' column to datetime
            df["publishedAt"] = pd.to_datetime(df["publishedAt"])

            return df

        except FileNotFoundError:
            channel_data = self.generate_channel_data()
            if channel_data is None:
                return
            return self.store_data(channel_data)

    def generate_channel_data(self):
        """
        Generate all video data for the channel using the Youtube API.
        """

        self.logger.info("Generating channel data")
        playlist_id = self.get_uploads_playlist_id()
        if playlist_id is None:
            return None
        videos_ids = self.get_videos_from_playlist(playlist_id)
        video_details = self.get_video_details(videos_ids)
        channel_data = self.create_dataframe(video_details)

        return channel_data

    def store_data(self, df):
        self.logger.info("Storing channel data")
        df = df.sort_values(by=["publishedAt"])
        df.to_csv("data/channel_data.csv", sep=",")
        return df

    def create_dataframe(self, video_details):
        # Dictionary to store video data
        videos_info = {
            "id": [],
            "title": [],
            "publishedAt": [],
            "duration": [],
            "currentMean": [],
            "url": [],
        }
        # Creating the dataframe
        duration_sum = 0
        for i, video in enumerate(reversed(video_details)):
            duration = self.duration_to_seconds(video["duration"])
            if duration <= 0:
                continue
            duration_sum += duration
            id = video["id"]
            title = video["title"]
            publishedAt = pd.to_datetime(video["publishedAt"])
            currentMean = round(duration_sum / (i + 1), 2)
            url = f"https://www.youtube.com/watch?v={id}"
            videos_info["id"].append(id)
            videos_info["title"].append(title)
            videos_info["publishedAt"].append(publishedAt)
            videos_info["duration"].append(duration)
            videos_info["currentMean"].append(currentMean)
            videos_info["url"].append(url)

        df = pd.DataFrame(data=videos_info)
        df = df.drop_duplicates(subset="id")
        df = df.sort_values(by=["publishedAt"])
        return df

    # Função para converter duração ISO 8601 para segundos
    def duration_to_seconds(self, duration):
        """Converte uma duração no formato ISO 8601 para segundos."""
        pattern = re.compile(r"PT(?:(\d+)H)?(?:(\d+)M)?(?:(\d+)S)?")
        match = pattern.match(duration)

        if not match:
            return 0

        hours, minutes, seconds = match.groups()

        hours = int(hours or 0)
        minutes = int(minutes or 0)
        seconds = int(seconds or 0)

        return hours * 3600 + minutes * 60 + seconds

    # Obter o ID da playlist de uploads do canal
    def get_uploads_playlist_id(self):
        try:
            response = (
                self.youtube.channels()
                .list(part="contentDetails", id=self.channel_id)
                .execute()
            )

            return response["items"][0]["contentDetails"]["relatedPlaylists"]["uploads"]
        except googleapiclient.errors.HttpError as err:
            self.logger.error(f"Error requesting google api: {err}")
            self.config.wait(self.logger, 12 * 60, 8 * 60)
            return None

    # Obter todos os vídeos da playlist de uploads
    def get_videos_from_playlist(self, playlist_id):
        videos = []
        next_page_token = None

        while True:
            response = (
                self.youtube.playlistItems()
                .list(
                    part="snippet",
                    playlistId=playlist_id,
                    maxResults=50,
                    pageToken=next_page_token,
                )
                .execute()
            )

            videos.extend(
                item["snippet"]["resourceId"]["videoId"] for item in response["items"]
            )
            next_page_token = response.get("nextPageToken")
            if not next_page_token:
                break

        return videos

    # Obter detalhes dos vídeos
    def get_video_details(self, video_ids):
        details = []
        for i in range(0, len(video_ids), 50):
            response = (
                self.youtube.videos()
                .list(part="snippet,contentDetails", id=",".join(video_ids[i : i + 50]))
                .execute()
            )

            for item in response["items"]:
                video_data = {
                    "id": item["id"],
                    "title": item["snippet"]["title"],
                    "publishedAt": item["snippet"]["publishedAt"],
                    "duration": item["contentDetails"]["duration"],
                }
                details.append(video_data)
        return details
