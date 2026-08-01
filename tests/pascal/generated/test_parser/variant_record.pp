unit u;
interface
type
  TVal = record
    tag : integer;
    case k : integer of
      1 : (i : integer);
      2 : (r : real);
  end;
implementation
end.
