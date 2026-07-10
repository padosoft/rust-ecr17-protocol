import { COMMANDS, type CommandDef } from "../ecr17/commands";

interface Props {
  disabled: boolean;
  onPick: (cmd: CommandDef) => void;
}

export function CommandPalette({ disabled, onPick }: Props) {
  return (
    <section className="panel" data-testid="command-palette">
      <h2 className="panel__title">Commands</h2>
      <div className="cmd-grid">
        {COMMANDS.map((cmd) => (
          <button
            key={cmd.key}
            type="button"
            className={`cmd${cmd.danger ? " cmd--danger" : ""}`}
            onClick={() => onPick(cmd)}
            disabled={disabled}
            data-testid={`cmd-${cmd.key}`}
          >
            <span className="cmd__letter" aria-hidden="true">
              {cmd.letter}
            </span>
            <span className="cmd__label">{cmd.label}</span>
          </button>
        ))}
      </div>
    </section>
  );
}
