import time
import os
import requests
import litellm
from litellm import completion
import json

class ClaudeGen:

    def __init__(self, model):

        litellm.vertex_location = 'us-east5'

        with open(os.environ['VERTEX_AI_JSON'], 'r') as file:
            self.vertex_credentials = json.dumps(json.load(file))
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
        from .. import ModelException

        if top_k != 1 and temperature == 0:
            raise ModelException("Top k sampling requires a non-zero temperature")
        
        count = 0
        while True:
            try:
                response = completion(
                    model=f"vertex_ai/{self.model}",
                    messages=messages,
                    temperature=temperature,
                    n=top_k,
                    vertex_credentials=self.vertex_credentials,
                    max_tokens=64000
                )
                break
            except (litellm.BadRequestError, litellm.AuthenticationError,
                        litellm.NotFoundError, litellm.UnprocessableEntityError) as e:
                    raise ModelException(f"Encountered an error with Vertex AI API {e}")
            except (litellm.RateLimitError, litellm.InternalServerError, litellm.APIConnectionError) as e:
                count += 1
                if count >= 5:
                    raise ModelException("Vertex AI API: Too many retries")
                print(f"Vertex AI Error {e}. Waiting 10 seconds and retrying")
                time.sleep(10)
            except Exception as e:
                raise ModelException(f"Vertex AI API Error: {e}")
        
        return [choice['message']['content'] for choice in response['choices']]