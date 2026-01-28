1. The "Librarian" Persona
   The AI doesn't just "search"—it acts as a Senior Digital Asset Manager. Its goal is to bridge the gap between a vague user request and a highly specific search engine query.

Role: Expert in brand identity, industrial design, and web assets.

Mission: Identify the most likely intent (Logo vs. Product Photo vs. Technical Drawing) and generate 3-5 optimized search "hooks."

2. System Prompt Blueprint
   This is the core instruction sent to the LLM (GPT-4o, Claude, or a local Llama 3 model) before the user's input.

System Prompt: "You are an AI Asset Scout. Your task is to take a user's short input and expand it into a structured JSON object for image search engines.

Rules:

Identify the Object (e.g., Braun Player).

Identify the Category (Logo, Product, Architecture, etc.).

Generate Search Terms that include technical modifiers like 'high-res', 'transparent PNG', 'SVG vector', 'white background', or 'studio lighting'.

If the input is a brand, prioritize official logos and current brand identity guidelines.

Output Format: { "intent": "string", "refined_queries": ["query 1", "query 2", "query 3"], "suggested_filters": {"color": "string", "type": "string"} }"

3. Transformation Examples
   How the engine handles common inputs:

User Input AI Reasoning Optimized Search Queries
"BMW Logo" User needs a brand asset. Prioritize vectors.

1. "BMW official logo SVG transparent"

2. "BMW 2024 roundel logo high-res png"

3. "BMW brand guidelines logo assets"

"Braun Player" Likely the SK4 "Snow White's Coffin." Needs design reference.

1. "Braun SK4 record player Dieter Rams studio shot"

2. "Vintage Braun vinyl player white background"

3. "Braun audio system high resolution product photo"

"Coffee Icon" User is building a UI. Needs icons/illustrations.

1. "Coffee cup icon flat vector set"

2. "Minimalist coffee line art SVG"

3. "Modern coffee shop icon transparent"

4. Multi-Provider Support
   The app will support two tiers of AI logic:

Cloud (OpenAI/Anthropic): Best for complex reasoning and "hallucination-free" brand identification.

Local (Ollama/Llama 3): Ideal for "Open Source" purists who want to run the tool 100% offline with zero API costs.

5. Metadata Enrichment
   The AI doesn't just help find the image; it helps the app understand it.

Auto-Tagging: When an image is found, the AI can guess tags like "Minimalist," "1960s," or "Monochrome."

License Guessing: Based on the source URL (e.g., Unsplash), the AI can flag the asset as "Safe for Commercial Use."
