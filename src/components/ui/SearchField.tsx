import type { ChangeEventHandler } from "react";

type SearchFieldProps = {
  label: string;
  placeholder: string;
  value?: string;
  onChange?: ChangeEventHandler<HTMLInputElement>;
};

export function SearchField({ label, placeholder, value, onChange }: SearchFieldProps) {
  return (
    <label className="search-field">
      <span className="search-field__icon" aria-hidden="true">⌕</span>
      <span className="sr-only">{label}</span>
      <input
        aria-label={label}
        placeholder={placeholder}
        value={value}
        onChange={onChange}
      />
    </label>
  );
}
