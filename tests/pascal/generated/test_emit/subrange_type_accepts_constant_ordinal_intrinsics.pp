unit u;
interface
type
  tcgloc = (loc_invalid, loc_void, loc_creference, loc_reference);
  tcgnonrefloc = low(tcgloc)..pred(loc_creference);
  tlocation = record
    loc: tcgloc;
  end;
procedure reset(var l: tlocation; lt: tcgnonrefloc);
implementation
procedure reset(var l: tlocation; lt: tcgnonrefloc);
begin
  l.loc := lt;
end;
end.
