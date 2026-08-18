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

  it("derives account summaries from provider metrics", () => {
    render(<App />);
    expect(screen.getByText("$74.75 · ¥82.31")).toBeInTheDocument();
    expect(screen.getAllByText("$18.42")).toHaveLength(2);
    expect(screen.getByText("4/4")).toBeInTheDocument();
    expect(document.body.textContent).not.toContain("$42.18");
  });

  it("shows the stored history basis in account details", () => {
    render(<App />);
    fireEvent.click(screen.getByText("OpenAI API").closest("article")!);
    expect(screen.getByRole("heading", { name: "Daily spend" })).toBeInTheDocument();
    expect(screen.getByText("Provider history buckets")).toBeInTheDocument();
  });

  it("edits an account name without replacing its credential", () => {
    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: "Accounts" }));
    fireEvent.click(screen.getByRole("button", { name: "Edit OpenAI API" }));
    const name = screen.getByLabelText("Account name");
    fireEvent.change(name, { target: { value: "Primary OpenAI" } });
    fireEvent.click(screen.getByRole("button", { name: "Save changes" }));
    expect(screen.getByRole("heading", { name: "Primary OpenAI" })).toBeInTheDocument();
  });

  it("keeps Google AI Studio API and Gemini CLI as separate account types", () => {
    render(<App />);
    expect(screen.getByText("Google AI Studio API")).toBeInTheDocument();
    expect(screen.getByText("Google AI Pro")).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "Accounts" }));
    expect(screen.getByRole("table", { name: "API platform accounts" })).toHaveTextContent("Google AI Studio API");
    expect(screen.getByRole("table", { name: "Subscription accounts" })).toHaveTextContent("Google AI Pro");
  });

  it("edits manual subscription metadata independently of automatic quota", () => {
    render(<App />);
    fireEvent.click(screen.getByRole("button", { name: "Accounts" }));
    fireEvent.click(screen.getByRole("button", { name: "Edit ChatGPT Plus" }));
    fireEvent.change(screen.getByLabelText("Plan name · Manual"), { target: { value: "ChatGPT Pro" } });
    fireEvent.click(screen.getByRole("button", { name: "Save changes" }));
    expect(screen.getByRole("heading", { name: "ChatGPT Plus" })).toBeInTheDocument();
    expect(document.body.textContent).toContain("ChatGPT Pro");
  });
});
