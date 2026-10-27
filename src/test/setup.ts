import "@testing-library/jest-dom/vitest";
import { afterEach } from "vitest";
import { cleanup } from "@testing-library/react";
afterEach(cleanup);
// jsdom lacks ResizeObserver; the editor only uses it to size the preview.
class RO { observe() {} unobserve() {} disconnect() {} }
(globalThis as unknown as { ResizeObserver: unknown }).ResizeObserver ??= RO;

// jsdom has no 2D canvas: previews that draw (scenes, colour picking) get no context and skip drawing quietly.
HTMLCanvasElement.prototype.getContext = (() => null) as unknown as HTMLCanvasElement["getContext"];
