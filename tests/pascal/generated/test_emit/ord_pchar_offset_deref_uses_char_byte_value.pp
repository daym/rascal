unit u;
interface
procedure run(p : pchar);
implementation
procedure run(p : pchar);
begin
  if (ord((p+1)^)=187) and (ord((p+2)^)=191) then begin end;
end;
end.
