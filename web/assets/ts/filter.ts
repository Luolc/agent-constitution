// The filter of the rules' list, without the page: the values chosen of
// each field, which rules they match, and how the address holds them.
// Within a field a rule needs one of the values chosen, and it needs
// that in every field; a field with all its values chosen filters
// nothing. The address holds only the fields whose choice is not the
// default one, each as its values, keys of [a-z0-9-], joined by commas
// (`status=active,trial,draft&domain=13,14`; `status=` chooses none).
// The address before this, from a form of check boxes, had `filter=1`
// and one parameter per value checked (`filter=1&status=active&
// status=trial`), a field without one having none checked, except the
// one box of `default`, which chose only `on` when checked.

// The values chosen of each field, by field.
export type Filter = Map<string, Set<string>>;

// The values of each field the page offers, in their order, by field.
export type Fields = ReadonlyMap<string, readonly string[]>;

// The values of a rule for a field: one for most, both uses for some.
export type Values = (field: string) => readonly string[];

// The fields that filter: those with a value not chosen.
function filtering(filter: Filter, fields: Fields): string[] {
  return [...fields].filter(([f, offered]) => (filter.get(f)?.size ?? 0) < offered.length).map(([f]) => f);
}

// The fields whose values differ from the default filter's, the number
// on the filter's button.
export function differing(filter: Filter, fields: Fields, defaults: Filter): string[] {
  return write(filter, fields, defaults).map((param) => param.slice(0, param.indexOf("=")));
}

// Whether a rule's values meet the filter.
export function matches(filter: Filter, fields: Fields, values: Values): boolean {
  return filtering(filter, fields).every((f) => values(f).some((v) => filter.get(f)?.has(v)));
}

// The filter of the address; a field it leaves out takes the default
// filter's values, a value the page does not offer is left out.
export function read(query: URLSearchParams, fields: Fields, defaults: Filter): Filter {
  const legacy = query.get("filter") === "1";
  const filter: Filter = new Map();
  for (const [f, offered] of fields) {
    let chosen: string[];
    if (legacy && f === "default") {
      chosen = query.getAll(f).includes("on") ? ["on"] : [...offered];
    } else if (legacy || query.has(f)) {
      chosen = query.getAll(f).flatMap((v) => v.split(","));
    } else {
      chosen = [...(defaults.get(f) ?? offered)];
    }
    filter.set(f, new Set(offered.filter((v) => chosen.includes(v))));
  }
  return filter;
}

// The address's parameters for the filter: the fields whose values
// differ from the default filter's.
export function write(filter: Filter, fields: Fields, defaults: Filter): string[] {
  const params: string[] = [];
  for (const [f, offered] of fields) {
    const chosen = offered.filter((v) => filter.get(f)?.has(v));
    const given = offered.filter((v) => (defaults.get(f) ?? new Set(offered)).has(v));
    if (chosen.join(",") !== given.join(",")) params.push(`${f}=${chosen.join(",")}`);
  }
  return params;
}
