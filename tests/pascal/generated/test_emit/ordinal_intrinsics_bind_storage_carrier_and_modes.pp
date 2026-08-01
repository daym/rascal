unit u;
interface
function nextchecked(v : byte) : byte;
function prevwrapped(v : byte) : byte;
procedure mutate(var v : byte);
implementation
{$Q+}{$R+}
function nextchecked(v : byte) : byte;
begin nextchecked := succ(v); end;
{$Q-}{$R-}
function prevwrapped(v : byte) : byte;
begin prevwrapped := pred(v); end;
{$Q+}{$R+}
procedure mutate(var v : byte);
begin inc(v); dec(v); end;
end.
