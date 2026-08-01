unit u;
interface
const
  nextlabelnr : longint = 1;
procedure bump;
implementation
procedure bump;
begin
  nextlabelnr := nextlabelnr + 1;
end;
end.
