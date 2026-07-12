import { useEffect, useId, useMemo, useState } from "react";
import type { CommandDef, Field } from "../ecr17/commands";

interface Props {
  command: CommandDef | null;
  onClose: () => void;
  onSubmit: (key: string, params: Record<string, unknown>) => void;
}

type RawValues = Record<string, string | boolean>;

function initialValues(command: CommandDef): RawValues {
  const v: RawValues = {};
  for (const f of command.fields) {
    if (f.kind === "bool") {
      v[f.name] = false;
    } else if (f.kind === "enum") {
      v[f.name] = f.options?.[0]?.value ?? "";
    } else {
      v[f.name] = "";
    }
  }
  return v;
}

// Converts a raw form value to the typed value the command expects. `raw` may be `undefined`
// on the first render after `command` becomes non-null, before the init effect runs — treat
// it as empty so we never coerce the literal string "undefined".
function coerce(field: Field, raw: string | boolean | undefined): unknown {
  switch (field.kind) {
    case "bool":
      return raw === true;
    case "money": {
      // Entered in euros; sent as integer cents.
      const euros = Number.parseFloat(String(raw ?? ""));
      return Number.isFinite(euros) ? Math.round(euros * 100) : 0;
    }
    case "number": {
      const n = Number.parseInt(String(raw ?? ""), 10);
      return Number.isFinite(n) ? n : 0;
    }
    default:
      return String(raw ?? "");
  }
}

function isMissing(field: Field, raw: string | boolean | undefined): boolean {
  if (!field.required || field.kind === "bool") {
    return false;
  }
  // A required amount must coerce to a positive integer. We validate the *converted* value
  // (cents for money), not the raw euros, so a sub-cent entry like "0.004" — which rounds to
  // 0 cents — is rejected rather than silently sent as a zero-amount financial transaction.
  if (field.kind === "money" || field.kind === "number") {
    const value = coerce(field, raw);
    return typeof value !== "number" || value <= 0;
  }
  return String(raw ?? "").trim() === "";
}

export function CommandParamsSheet({ command, onClose, onSubmit }: Props) {
  const [values, setValues] = useState<RawValues>({});
  const baseId = useId();

  useEffect(() => {
    if (command) {
      setValues(initialValues(command));
    }
  }, [command]);

  // Close on Escape (keyboard equivalent of the backdrop click).
  useEffect(() => {
    if (!command) {
      return;
    }
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        onClose();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [command, onClose]);

  const missing = useMemo(
    () => (command ? command.fields.filter((f) => isMissing(f, values[f.name])) : []),
    [command, values],
  );

  if (!command) {
    return null;
  }

  const submit = () => {
    if (missing.length > 0) {
      return;
    }
    const params: Record<string, unknown> = {};
    for (const f of command.fields) {
      params[f.name] = coerce(f, values[f.name]);
    }
    onSubmit(command.key, params);
    onClose();
  };

  const fieldId = (name: string) => `${baseId}-${name}`;

  return (
    <div className="sheet-backdrop" data-testid="params-sheet" role="presentation">
      {/* Full-screen button behind the dialog: a click outside the panel closes it. */}
      <button
        type="button"
        className="sheet-backdrop__hit"
        aria-label="Close dialog"
        onClick={onClose}
        data-testid="sheet-backdrop"
      />
      <div className="sheet" role="dialog" aria-modal="true" aria-label={command.label}>
        <div className="sheet__header">
          <h2 className="sheet__title">
            <span className={`cmd__letter${command.danger ? " cmd__letter--danger" : ""}`}>
              {command.letter}
            </span>
            {command.label}
          </h2>
          <button
            type="button"
            className="btn btn--sm"
            onClick={onClose}
            aria-label="Close"
            data-testid="sheet-close"
          >
            ✕
          </button>
        </div>

        <div className="sheet__body">
          {command.fields.map((f) => (
            <div key={f.name} className="field" data-testid={`param-${f.name}`}>
              <label className="field__label" htmlFor={fieldId(f.name)}>
                {f.label}
                {f.required ? <span className="req"> *</span> : null}
              </label>
              {f.kind === "bool" ? (
                <input
                  id={fieldId(f.name)}
                  type="checkbox"
                  checked={values[f.name] === true}
                  onChange={(e) => {
                    const checked = e.currentTarget.checked;
                    setValues((v) => ({ ...v, [f.name]: checked }));
                  }}
                  data-testid={`input-${f.name}`}
                />
              ) : f.kind === "enum" ? (
                <select
                  id={fieldId(f.name)}
                  className="input"
                  value={String(values[f.name] ?? "")}
                  onChange={(e) => {
                    const val = e.currentTarget.value;
                    setValues((v) => ({ ...v, [f.name]: val }));
                  }}
                  data-testid={`input-${f.name}`}
                >
                  {f.options?.map((o) => (
                    <option key={o.value} value={o.value}>
                      {o.label}
                    </option>
                  ))}
                </select>
              ) : (
                <input
                  id={fieldId(f.name)}
                  className="input"
                  type={f.kind === "money" || f.kind === "number" ? "number" : "text"}
                  step={f.kind === "money" ? "0.01" : f.kind === "number" ? "1" : undefined}
                  value={String(values[f.name] ?? "")}
                  placeholder={f.placeholder}
                  onChange={(e) => {
                    const val = e.currentTarget.value;
                    setValues((v) => ({ ...v, [f.name]: val }));
                  }}
                  data-testid={`input-${f.name}`}
                />
              )}
            </div>
          ))}
        </div>

        <div className="sheet__footer">
          <button
            type="button"
            className={`btn btn--primary${command.danger ? " btn--danger" : ""}`}
            onClick={submit}
            disabled={missing.length > 0}
            data-testid="sheet-submit"
          >
            Run {command.label}
          </button>
        </div>
      </div>
    </div>
  );
}
