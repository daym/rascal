unit u;
interface
procedure f(x : longint); overload;
procedure f(x : longint = 5); overload;
procedure run;
implementation
procedure f(x : longint); begin end;
procedure f(x : longint = 5); begin end;
procedure run;
begin
  f(7);
end;
end.
