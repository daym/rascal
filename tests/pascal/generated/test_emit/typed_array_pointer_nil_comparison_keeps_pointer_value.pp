unit u;
interface
type
  tbuf = array[0..3] of char;
  pbuf = ^tbuf;
function empty(p : pbuf) : boolean;
implementation
function empty(p : pbuf) : boolean;
begin
  empty := p = nil;
end;
end.
