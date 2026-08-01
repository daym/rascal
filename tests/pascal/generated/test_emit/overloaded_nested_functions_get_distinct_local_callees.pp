unit u;
interface
function outer : longint;
implementation
function outer : longint;
  function pick(n : longint) : longint;
  begin
    pick := n;
  end;
  function pick(b : boolean) : longint;
  begin
    if b then pick := 1 else pick := 0;
  end;
begin
  outer := pick(5) + pick(true);
end;
end.
