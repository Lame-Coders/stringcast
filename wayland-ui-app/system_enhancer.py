import os
import sys
from dotenv import load_dotenv

# Load variables from the .env file into the environment
load_dotenv()

try:
    import google.generativeai as genai
except ImportError:
    print("Warning: gemini library not found. Install with: pip install google-generativeai")

try:
    from openai import OpenAI
except ImportError:
    print("Warning: openai library not found. Install with: pip install openai")

# --- Configuration ---
DEFAULT_KEYS = {
    "Gemini": os.environ.get("GEMINI_API_KEY"),
    "OpenAI": os.environ.get("OPENAI_API_KEY"),
}
PROVIDER_ORDER = ["Gemini", "OpenAI"]

# --- API Client Setup ---
def initialize_clients():
    """Initializes API clients based on environment variables."""
    clients = {}

    gemini_key = DEFAULT_KEYS["Gemini"]
    if gemini_key:
        try:
            genai.configure(api_key=gemini_key)
            # Store the model identifier we want to use
            clients["Gemini"] = "gemini-2.5-flash" 
            print("✅ Client initialized for Gemini.")
        except Exception as e:
            print(f"❌ Error initializing Gemini client: {e}")

    openai_key = DEFAULT_KEYS["OpenAI"]
    if openai_key:
        try:
            clients["OpenAI"] = OpenAI(api_key=openai_key)
            print("✅ Client initialized for OpenAI.")
        except Exception as e:
            print(f"❌ Error initializing OpenAI client: {e}")

    return clients

AI_CLIENTS = initialize_clients()

def construct_prompt(function_name: str) -> str:
    """Builds the system instruction based on the selected enhancement task."""
    print(f"    [Prompting] Building context for: {function_name}...")
    if function_name == "Improve Grammar & Flow":
        return "You are a master copyeditor. Improve the grammar, sentence structure, and overall flow of the following text while strictly maintaining the original meaning. Return ONLY the perfected text, without any conversational preamble."
    elif function_name == "Enhance Tone to Professional":
        return "Rewrite the following text to adopt a formal, highly professional, and measured tone suitable for a corporate memo. Do not alter the core message. Return ONLY the rewritten text."
    elif function_name.startswith("Translate"):
        target_lang = function_name.split("to ")[-1].strip()
        return f"Translate the following text accurately and naturally into {target_lang}. Return ONLY the translated text."
    else:
        return "Please process this text with maximum clarity and sophistication. Return ONLY the processed text."

def call_llm_api(provider_name: str, text_to_enhance: str, system_instruction: str) -> str:
    """CORE API INTERACTION FUNCTION."""
    print(f"    -> Sending request to {provider_name}...")

    if provider_name == "Gemini":
        model_name = AI_CLIENTS.get("Gemini")
        if not model_name:
             raise ValueError("Gemini client not initialized.")
        
        # Gemini handles system instructions via the GenerativeModel constructor or by prepending
        model = genai.GenerativeModel(model_name)
        full_prompt = f"{system_instruction}\n\nText to process:\n{text_to_enhance}"
        response = model.generate_content(full_prompt)
        return response.text.strip()

    elif provider_name == "OpenAI":
        client = AI_CLIENTS.get("OpenAI")
        if not client:
             raise ValueError("OpenAI client not initialized.")
             
        response = client.chat.completions.create(
            model="gpt-4o-mini",
            messages=[
                {"role": "system", "content": system_instruction},
                {"role": "user", "content": text_to_enhance}
            ]
        )
        return response.choices[0].message.content.strip()

    raise NotImplementedError(f"API call logic for {provider_name} is not implemented.")

def try_transform_text(text_to_enhance: str, function_name: str) -> str:
    """Attempts to transform text using multiple APIs in sequence."""
    system_instruction = construct_prompt(function_name)

    for provider_name in PROVIDER_ORDER:
        if provider_name not in AI_CLIENTS:
             print(f"⚠️ Skipping {provider_name} (Not configured or missing API key).")
             continue

        print(f"\n--- Attempting connection using {provider_name} ---")
        try:
            result = call_llm_api(provider_name, text_to_enhance, system_instruction)
            print(f"✅ Success using {provider_name}!")
            return result
        except Exception as e:
            print(f"❌ Failed using {provider_name}: {e.__class__.__name__} - {e}. Attempting fallback...")
            continue

    return "\n⚠️ FATAL: ALL configured APIs failed to process the request."

# ====================================================================
# REAL EXECUTION EXAMPLES
# ====================================================================
if __name__ == "__main__":
    print("\n=========================================")
    print("RUNNING REAL TEST: Improve Grammar")
    print("=========================================")
    sample_text = "i dont knwo whats happening but the code is crash."
    
    result = try_transform_text(sample_text, "Improve Grammar & Flow")
    
    print("\n[ORIGINAL]:", sample_text)
    print("[ENHANCED]:", result)