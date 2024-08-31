import pandas as pd
import re
import logging

import googleapiclient.discovery


class YoutubeAPI:
    def __init__(self, developer_key, channel_id):
        logging.basicConfig(level=logging.INFO)
        self.logger = logging.getLogger(__name__)

        self.api_service_name = "youtube"
        self.api_version = "v3"
        self.channel_id = channel_id
        self.developer_key = developer_key
        self.youtube = googleapiclient.discovery.build(
            self.api_service_name, self.api_version, developerKey = developer_key)

    def store_data(self, channel_data):
        self.logger.info("Storing channel data")
        channel_data.to_csv("channel_data.csv", sep=",", index=False)

    def get_latest_video(self):
        playlist_id = self.get_uploads_playlist_id()

        # Obter o ID do último vídeo da playlist de uploads
        response = self.youtube.playlistItems().list(
            part="snippet",
            playlistId=playlist_id,
            maxResults=1
        ).execute()

        if not response['items']:
            self.logger.info("No video found.")
            return None

        latest_video_id = response['items'][0]['snippet']['resourceId']['videoId']

        latest_video_details = self.get_video_details([latest_video_id])

        if latest_video_details:
            latest_video_dataframe = self.create_dataframe(latest_video_details)
            return latest_video_dataframe
        else:
            self.logger.info("Não foi possível obter detalhes do último vídeo.")
            return None

    def create_dataframe(self, video_details):
        # Dictionary to store video data
        videos_info = {
            'id': [],
            'title': [],
            'publishedAt': [],
            'duration': [],
            'currentMean': [],
            'url': [],
        }
        # Creating the dataframe
        duration_sum = 0
        for i, video in enumerate(reversed(video_details)):
            duration = self.duration_to_seconds(video['duration'])
            if duration <= 0:
                continue
            duration_sum += duration
            id = video['id']
            title = video['title']
            publishedAt = video['publishedAt']
            currentMean = round(duration_sum / (i + 1), 2)
            url = f"https://www.youtube.com/watch?v={id}"
            videos_info['id'].append(id)
            videos_info['title'].append(title)
            videos_info['publishedAt'].append(publishedAt)
            videos_info['duration'].append(duration)
            videos_info['currentMean'].append(currentMean)
            videos_info['url'].append(url)

        return pd.DataFrame(data=videos_info)

    def generate_channel_data(self):
        """
        Generate all video data for the channel using the Youtube API.
        """

        self.logger.info("Generating channel data")
        playlist_id = self.get_uploads_playlist_id()
        videos_ids = self.get_videos_from_playlist(playlist_id)
        video_details = self.get_video_details(videos_ids)
        channel_data = self.create_dataframe(video_details)

        return channel_data

    # Função para converter duração ISO 8601 para segundos
    def duration_to_seconds(self, duration):
        """Converte uma duração no formato ISO 8601 para segundos."""
        pattern = re.compile(r'PT(?:(\d+)H)?(?:(\d+)M)?(?:(\d+)S)?')
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
        response = self.youtube.channels().list(
            part="contentDetails",
            id=self.channel_id
        ).execute()

        playlist = response['items'][0]['contentDetails']['relatedPlaylists']
        return playlist['uploads']

    # Obter todos os vídeos da playlist de uploads
    def get_videos_from_playlist(self, playlist_id):
        videos = []
        next_page_token = None

        while True:
            response = self.youtube.playlistItems().list(
                part="snippet",
                playlistId=playlist_id,
                maxResults=50,
                pageToken=next_page_token
            ).execute()

            videos.extend(
                item['snippet']['resourceId']['videoId']
                for item in response['items']
            )
            next_page_token = response.get('nextPageToken')
            if not next_page_token:
                break

        return videos

    # Obter detalhes dos vídeos
    def get_video_details(self, video_ids):
        details = []
        for i in range(0, len(video_ids), 50):
            response = self.youtube.videos().list(
                part="snippet,contentDetails",
                id=','.join(video_ids[i:i+50])
            ).execute()

            for item in response['items']:
                video_data = {
                    'id': item['id'],
                    'title': item['snippet']['title'],
                    'publishedAt': item['snippet']['publishedAt'],
                    'duration': item['contentDetails']['duration']
                }
                details.append(video_data)
        return details
