unit u;
interface
type
  tnames = array[0..1] of pchar;
const
  Names : tnames = ('tc_none', '');
function miss : pchar;
implementation
function miss : pchar;
begin
  miss := '';
end;
end.
