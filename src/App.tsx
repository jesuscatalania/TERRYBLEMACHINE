import { invoke } from "@tauri-apps/api/core";
import { type FormEvent, useState } from "react";

function App() {
  const [greetMsg, setGreetMsg] = useState("");
  const [name, setName] = useState("");

  async function greet(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setGreetMsg(await invoke("greet", { name }));
  }

  return (
    <main className="flex min-h-screen flex-col items-center justify-center gap-8 bg-neutral-dark-900 p-10 text-neutral-dark-100">
      <h1 className="font-display text-4xl font-bold tracking-tight text-neutral-dark-50">
        Welcome to Tauri + React
      </h1>

      <p className="text-sm text-neutral-dark-300">
        TERRYBLEMACHINE — local AI design tool. Foundation scaffolding.
      </p>

      <form onSubmit={greet} className="flex w-full max-w-md gap-2">
        <label htmlFor="greet-input" className="sr-only">
          Name
        </label>
        <input
          id="greet-input"
          name="name"
          onChange={(e) => setName(e.currentTarget.value)}
          placeholder="Enter a name..."
          aria-label="Name"
          className="flex-1 rounded-md border border-neutral-dark-600 bg-neutral-dark-800 px-3 py-2 text-sm text-neutral-dark-100 placeholder:text-neutral-dark-400 focus:border-accent-500 focus:outline-none"
        />
        <button
          type="submit"
          className="rounded-md bg-accent-500 px-4 py-2 text-sm font-medium text-white shadow-soft transition hover:bg-accent-600 focus:outline-none focus:ring-2 focus:ring-accent-400"
        >
          Greet
        </button>
      </form>
      {greetMsg ? <p className="text-sm text-neutral-dark-200">{greetMsg}</p> : null}
    </main>
  );
}

export default App;
