import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { NewProjectDialog } from "@/components/projects/NewProjectDialog";

describe("NewProjectDialog", () => {
  it("does not render when open=false", () => {
    render(<NewProjectDialog open={false} onClose={() => {}} onCreate={vi.fn()} />);
    expect(screen.queryByRole("dialog")).toBeNull();
  });

  it("renders the form fields when open", () => {
    render(<NewProjectDialog open={true} onClose={() => {}} onCreate={vi.fn()} />);
    expect(screen.getByLabelText(/name/i)).toBeInTheDocument();
    expect(screen.getByLabelText(/module/i)).toBeInTheDocument();
    expect(screen.getByLabelText(/description/i)).toBeInTheDocument();
  });

  it("disables the Create button when name is empty", () => {
    render(<NewProjectDialog open={true} onClose={() => {}} onCreate={vi.fn()} />);
    expect(screen.getByRole("button", { name: /create/i })).toBeDisabled();
  });

  it("calls onCreate with the typed values and closes on success", async () => {
    const user = userEvent.setup();
    const onCreate = vi.fn().mockResolvedValue(undefined);
    const onClose = vi.fn();
    render(
      <NewProjectDialog
        open={true}
        onClose={onClose}
        onCreate={onCreate}
        defaultModule="graphic2d"
      />,
    );

    await user.type(screen.getByLabelText(/name/i), "My Cool Thing");
    await user.type(screen.getByLabelText(/description/i), "A demo");

    await user.click(screen.getByRole("button", { name: /create/i }));

    expect(onCreate).toHaveBeenCalledWith({
      name: "My Cool Thing",
      module: "graphic2d",
      description: "A demo",
    });
    expect(onClose).toHaveBeenCalled();
  });

  it("displays an error when onCreate rejects", async () => {
    const user = userEvent.setup();
    const onCreate = vi.fn().mockRejectedValue({ kind: "InvalidName", detail: "bad name" });
    render(<NewProjectDialog open={true} onClose={() => {}} onCreate={onCreate} />);
    await user.type(screen.getByLabelText(/name/i), "whatever");
    await user.click(screen.getByRole("button", { name: /create/i }));
    expect(await screen.findByRole("alert")).toHaveTextContent(/bad name/);
  });
});
