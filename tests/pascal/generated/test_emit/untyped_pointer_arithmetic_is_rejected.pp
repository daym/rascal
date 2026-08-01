unit u;
interface
function step(p : pointer; n : ptrint) : pointer;
implementation
function step(p : pointer; n : ptrint) : pointer;
begin step := p + n; end;
end.
