unit u;
interface
{$packenum 1}
type
  tsmall = (a, b, c);
  trec = packed record
    hi : word;
    lo : tsmall;
    kind : tsmall;
  end;
implementation
end.
