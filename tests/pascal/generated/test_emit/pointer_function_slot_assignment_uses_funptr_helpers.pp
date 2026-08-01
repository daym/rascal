unit u;
interface
procedure demo;
implementation
var
  oldexit : pointer;
procedure myexit;
begin
  exitproc := oldexit;
end;
procedure demo;
begin
  oldexit := exitproc;
  exitproc := @myexit;
end;
end.
