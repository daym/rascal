unit u;
interface
const
  IF_SM = 1;
  IF_SM2 = 2;
procedure check(flags : longint);
implementation
procedure check(flags : longint);
begin
  if (flags and (IF_SM or IF_SM2)) <> 0 then writeln(flags);
end;
end.
