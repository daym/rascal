unit u;
interface
procedure f(a : longint = 1; b : shortstring = 'x'); overload;
procedure f(a : longint;     b : shortstring = 'x'); overload;
procedure run;
implementation
procedure f(a : longint = 1; b : shortstring = 'x'); begin end;
procedure f(a : longint;     b : shortstring = 'x'); begin end;
procedure run;
begin
  f(7, 'y');
end;
end.
