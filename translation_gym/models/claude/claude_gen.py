import time
import os
import requests

class ClaudeGen:

    def __init__(self, model):

        self.api_key=os.environ['ANTHROPIC_API_KEY']
        self.model = model
    
    def gen(self, messages, temperature=0, top_k=1):
        '''
        messages: [{'role': 'system', 'content': 'You are an intelligent code assistant'},
                   {'role': 'user', 'content': 'Translate this program...'},
                   {'role': 'assistant', 'content': 'Here is the translation...'},
                   {'role': 'user', 'content': 'Do something else...'}]
                   
        <returned>: ['Sure, here is...',
                     'Okay, let me see...',
                     ...]
        len(<returned>) == top_k
        '''
        
        from models import ModelException

        if top_k != 1 and temperature == 0:
            raise ModelException("Top k sampling requires a non-zero temperature")

        new_messages = [message for message in messages if message['role'] != 'system']
        url = 'https://api.anthropic.com/v1/messages'

        retry_count = 0
        while True:
            try:
                response = requests.post(
                                url,
                                json={
                                    "model": self.model,
                                    "messages": new_messages,
                                    "temperature": temperature,
                                    "max_tokens": 4096
                                },
                                headers={
                                    'x-api-key': self.api_key,
                                    'content-type': 'application/json',
                                    "anthropic-version": "2023-06-01"
                                }
                            )
                if response.status_code != 200:
                    raise ModelException(response.json()['error']['message'])
                response = response.json()
                if len(response['content']) > 1:
                    raise ModelException("Claude returned multiple responses")
                break
            except Exception as e:
                retry_count += 1
                if retry_count >= 5:
                    raise ModelException(f"Claude API Error: {e}")
                print(f"Claude API Error: {e}. Waiting 10 seconds and retrying")
                time.sleep(10)
        
        if top_k > 1:
            return [response['content'][0]['text']] + self.gen(messages, temperature, top_k=top_k-1)
        else:
            return [response['content'][0]['text']]
