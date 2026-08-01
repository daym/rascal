unit u;
interface
type
  trec = packed record
    tag : byte;
    case byte of
      0 : (a : word; b : longint);
      1 : (c : longint);
  end;
implementation
end.
