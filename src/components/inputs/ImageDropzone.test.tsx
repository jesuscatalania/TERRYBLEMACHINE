import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import {
  ACCEPTED_MIME_TYPES,
  ImageDropzone,
  MAX_FILE_BYTES,
} from "@/components/inputs/ImageDropzone";

function makeFile(opts: { name: string; type?: string; size?: number }): File {
  // Build a File with a specific reported size by padding content.
  const size = opts.size ?? 4;
  const content = new Uint8Array(size);
  return new File([content], opts.name, {
    type: opts.type ?? "image/png",
    lastModified: 0,
  });
}

describe("ImageDropzone", () => {
  it("exposes accepted mime types for PNG/JPG/WebP/SVG/TIFF/PSD", () => {
    expect(ACCEPTED_MIME_TYPES).toEqual(
      expect.arrayContaining([
        "image/png",
        "image/jpeg",
        "image/webp",
        "image/svg+xml",
        "image/tiff",
        "image/vnd.adobe.photoshop",
      ]),
    );
  });

  it("renders the dropzone placeholder text", () => {
    render(<ImageDropzone onChange={vi.fn()} />);
    expect(screen.getByText(/drop.*image|drag.*image/i)).toBeInTheDocument();
  });

  it("selecting a PNG via the file input invokes onChange with the File", async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    render(<ImageDropzone onChange={onChange} />);
    const file = makeFile({ name: "logo.png", type: "image/png", size: 1024 });
    await user.upload(screen.getByLabelText(/upload image/i), file);
    expect(onChange).toHaveBeenCalledWith(file);
  });

  it("shows a warning when a file exceeds 50 MB", async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    render(<ImageDropzone onChange={onChange} />);
    const fake = makeFile({
      name: "huge.png",
      type: "image/png",
      size: MAX_FILE_BYTES + 1,
    });
    await user.upload(screen.getByLabelText(/upload image/i), fake);
    expect(await screen.findByRole("alert")).toHaveTextContent(/too large|max/i);
    expect(onChange).not.toHaveBeenCalled();
  });

  it("rejects non-accepted mime types with an alert", async () => {
    const user = userEvent.setup({ applyAccept: false });
    const onChange = vi.fn();
    render(<ImageDropzone onChange={onChange} />);
    const pdf = new File([new Uint8Array(4)], "doc.pdf", {
      type: "application/pdf",
    });
    await user.upload(screen.getByLabelText(/upload image/i), pdf);
    expect(await screen.findByRole("alert")).toHaveTextContent(/unsupported/i);
    expect(onChange).not.toHaveBeenCalled();
  });

  it("shows a filename preview after upload for non-renderable types (tiff/psd)", async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    render(<ImageDropzone onChange={onChange} />);
    const psd = makeFile({
      name: "artwork.psd",
      type: "image/vnd.adobe.photoshop",
      size: 2048,
    });
    await user.upload(screen.getByLabelText(/upload image/i), psd);
    expect(screen.getByText("artwork.psd")).toBeInTheDocument();
  });

  it("remove button clears the selection", async () => {
    const user = userEvent.setup();
    const onChange = vi.fn();
    render(<ImageDropzone onChange={onChange} />);
    const file = makeFile({ name: "a.png", type: "image/png", size: 10 });
    await user.upload(screen.getByLabelText(/upload image/i), file);
    expect(onChange).toHaveBeenLastCalledWith(file);
    await user.click(screen.getByRole("button", { name: /remove/i }));
    expect(onChange).toHaveBeenLastCalledWith(null);
  });
});
