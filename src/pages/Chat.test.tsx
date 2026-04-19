import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { ChatPage } from "@/pages/Chat";

describe("ChatPage", () => {
  it("renders header + empty state + input", () => {
    render(<ChatPage />);
    expect(screen.getByText(/CHAT · CLAUDE/i)).toBeInTheDocument();
    expect(screen.getByText(/Start a conversation/i)).toBeInTheDocument();
    expect(screen.getByPlaceholderText(/Type a message/i)).toBeInTheDocument();
    expect(screen.getByRole("log")).toBeInTheDocument();
  });
});
