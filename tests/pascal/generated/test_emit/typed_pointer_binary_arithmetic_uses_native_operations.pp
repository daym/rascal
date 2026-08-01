unit u;
interface
type pword = ^word;
function step(p : pword; n : ptrint) : pword;
function reverse(n : ptrint; p : pword) : pword;
function distance(a, b : pword) : ptrint;
implementation
function step(p : pword; n : ptrint) : pword;
begin step := p - n; end;
function reverse(n : ptrint; p : pword) : pword;
begin reverse := n + p; end;
function distance(a, b : pword) : ptrint;
begin distance := a - b; end;
end.
