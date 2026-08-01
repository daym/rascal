unit u;
interface
function outer : longint;
implementation
function outer : longint;
var
  t : longint;
  function read_expr(var exprType : longint; eval : boolean) : longint; forward;
  function read_factor(var factorType : longint; eval : boolean) : longint;
  begin
    read_factor := read_expr(factorType, eval);
  end;
  function read_expr(var exprType : longint; eval : boolean) : longint;
  begin
    read_expr := exprType;
  end;
begin
  t := 3;
  outer := read_factor(t, true);
end;
end.
