unit u;
interface
function hash_step(result : longword; ch : byte) : longword;
implementation
{$Q-}
function hash_step(result : longword; ch : byte) : longword;
begin
  hash_step := longword(longint(result shl 5) - longint(result)) xor ch;
end;
end.
