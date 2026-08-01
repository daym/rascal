unit u;
interface
type
  tcgloc = (loc_invalid, loc_void, loc_creference, loc_reference);
  tcgnonrefloc = low(tcgloc)..pred(loc_creference);
  tdefmap = array[1..ord(high(tcgloc))] of byte;
implementation
end.
