unit u;
interface
uses dep;
procedure take(b : byte); overload;
procedure take(i : longint); overload;
procedure run;
implementation
procedure take(b : byte); begin end;
procedure take(i : longint); begin end;
procedure run;
begin
  take(dep.b);
  take(dep.c);
end;
end.
