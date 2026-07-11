import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import type { CommandDef } from "../ecr17/commands";
import { CommandParamsSheet } from "./CommandParamsSheet";

const payCmd: CommandDef = {
  key: "pay",
  label: "Pay",
  letter: "P",
  danger: true,
  fields: [
    { name: "amountCents", label: "Amount (€)", kind: "money", required: true },
    { name: "receiptText", label: "Receipt text", kind: "text" },
  ],
};

describe("CommandParamsSheet", () => {
  it("renders nothing without a command", () => {
    const { container } = render(
      <CommandParamsSheet command={null} onClose={() => {}} onSubmit={() => {}} />,
    );
    expect(container).toBeEmptyDOMElement();
  });

  it("converts a money field from euros to integer cents on submit", async () => {
    const onSubmit = vi.fn();
    render(<CommandParamsSheet command={payCmd} onClose={() => {}} onSubmit={onSubmit} />);

    await userEvent.type(screen.getByTestId("input-amountCents"), "6.50");
    await userEvent.click(screen.getByTestId("sheet-submit"));

    expect(onSubmit).toHaveBeenCalledWith("pay", { amountCents: 650, receiptText: "" });
  });

  it("disables submit until a required field is filled", async () => {
    render(<CommandParamsSheet command={payCmd} onClose={() => {}} onSubmit={() => {}} />);
    const submit = screen.getByTestId("sheet-submit");
    expect(submit).toBeDisabled();
    await userEvent.type(screen.getByTestId("input-amountCents"), "10");
    expect(submit).toBeEnabled();
  });

  it("keeps submit disabled for a zero or negative amount (money-safety)", async () => {
    render(<CommandParamsSheet command={payCmd} onClose={() => {}} onSubmit={() => {}} />);
    const submit = screen.getByTestId("sheet-submit");
    const amount = screen.getByTestId("input-amountCents");

    await userEvent.type(amount, "0");
    expect(submit).toBeDisabled();

    await userEvent.clear(amount);
    await userEvent.type(amount, "-5");
    expect(submit).toBeDisabled();

    await userEvent.clear(amount);
    await userEvent.type(amount, "1");
    expect(submit).toBeEnabled();
  });

  it("keeps submit disabled for a sub-cent amount that rounds to zero (money-safety)", async () => {
    render(<CommandParamsSheet command={payCmd} onClose={() => {}} onSubmit={() => {}} />);
    const submit = screen.getByTestId("sheet-submit");
    // 0.004 € parses as positive but coerces to 0 cents — must not be submittable.
    await userEvent.type(screen.getByTestId("input-amountCents"), "0.004");
    expect(submit).toBeDisabled();
  });
});
