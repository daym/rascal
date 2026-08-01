unit u;
interface
procedure internalerror(i : longint);
implementation
uses dep;
procedure internalerror(i : longint);
begin
end;
initialization
  dep.internalerror := @internalerror;
end.
