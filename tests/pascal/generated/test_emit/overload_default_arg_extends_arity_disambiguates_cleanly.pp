unit u;
interface
procedure f(x : longint); overload;
procedure f(x : longint; y : shortstring); overload;
procedure run;
implementation
procedure f(x : longint); begin end;
procedure f(x : longint; y : shortstring); begin end;
procedure run;
begin
  f(7, 'y');
end;
end.
