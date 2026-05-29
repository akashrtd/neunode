import { describe, it, expect } from "vitest";
import type {
  ApiEnvelope,
  SuccessEnvelope,
  ErrorEnvelope,
} from "./types";

describe("types", () => {
  describe("ApiEnvelope type narrowing", () => {
    it("a success envelope has data and success=true", () => {
      const envelope: ApiEnvelope<string> = {
        data: "hello",
        success: true,
      };
      expect(envelope.success).toBe(true);
      if (envelope.success) {
        const success = envelope as SuccessEnvelope<string>;
        expect(success.data).toBe("hello");
      }
    });

    it("an error envelope has error and success=false", () => {
      const envelope: ApiEnvelope<string> = {
        error: { code: "NOT_FOUND", message: "not found" },
        success: false,
      };
      expect(envelope.success).toBe(false);
      if (!envelope.success) {
        const err = envelope as ErrorEnvelope;
        expect(err.error.code).toBe("NOT_FOUND");
        expect(err.error.message).toBe("not found");
      }
    });

    it("a success envelope with object data", () => {
      const envelope: ApiEnvelope<{ id: string }> = {
        data: { id: "abc-123" },
        success: true,
      };
      expect(envelope.success).toBe(true);
      if (envelope.success) {
        const success = envelope as SuccessEnvelope<{ id: string }>;
        expect(success.data.id).toBe("abc-123");
      }
    });

    it("an error envelope with HTTP error code", () => {
      const envelope: ApiEnvelope<never> = {
        error: { code: "HTTP_500", message: "Internal Server Error" },
        success: false,
      };
      expect(envelope.success).toBe(false);
      if (!envelope.success) {
        const err = envelope as ErrorEnvelope;
        expect(err.error.code).toBe("HTTP_500");
      }
    });
  });
});
