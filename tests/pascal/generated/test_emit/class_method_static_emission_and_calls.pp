unit u;
interface
type
  tx = class
    class function bar : integer;
    function foo : integer;
  end;
procedure demo(x : tx);
implementation
class function tx.bar : integer;
begin
  bar := 7;
end;
function tx.foo : integer;
begin
  foo := bar;
end;
procedure demo(x : tx);
begin
  x.bar;
  tx.bar;
end;
end.
