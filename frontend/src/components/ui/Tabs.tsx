type TabItem = {
  id: string;
  label: string;
};

type TabsProps = {
  label: string;
  items: TabItem[];
  value: string;
  onChange: (value: string) => void;
};

export function Tabs({ label, items, value, onChange }: TabsProps) {
  return (
    <div aria-label={label} className="tabs" role="tablist">
      {items.map((item) => {
        const selected = item.id === value;

        return (
          <button
            aria-selected={selected}
            className={selected ? "tabs__tab tabs__tab--active" : "tabs__tab"}
            key={item.id}
            onClick={() => onChange(item.id)}
            role="tab"
            type="button"
          >
            {item.label}
          </button>
        );
      })}
    </div>
  );
}
