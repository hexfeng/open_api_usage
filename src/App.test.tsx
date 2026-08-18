import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { App } from "./App";

describe("AI Usage Dashboard", () => {
  beforeEach(() => localStorage.clear());
  afterEach(cleanup);

  it("switches between list and card layouts", () => {
    render(<App />);
    const cards = screen.getByRole("button", { name: "Card layout" });
    fireEvent.click(cards);
    expect(cards).toHaveAttribute("aria-pressed", "true");
    expect(localStorage.getItem("aud-layout")).toBe("cards");
  });

  it("applies and remembers the light theme", () => {
    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: /Light/i }));
    expect(document.documentElement).toHaveAttribute("data-theme", "light");
    expect(localStorage.getItem("aud-theme")).toBe("light");
  });

  it("navigates to account and settings views", () => {
    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: "Accounts" }));
    expect(screen.getByRole("heading", { name: "Accounts" })).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Settings" }));
    expect(screen.getByRole("heading", { name: "Settings" })).toBeInTheDocument();
  });

  it("keeps API and subscription scope labels explicit", () => {
    render(<App />);
    expect(document.body.textContent).toContain("Organization");
    expect(document.body.textContent).toContain("Codex only");
    expect(document.body.textContent).toContain("Gemini CLI only");
  });
});
