unit u;
interface
type
  trec = record
    case outertag: integer of
      1: (
        case innertag: integer of
          0: (val: integer);
          1: (str: pansichar);
      );
      2: (other: integer);
  end;
implementation
end.
