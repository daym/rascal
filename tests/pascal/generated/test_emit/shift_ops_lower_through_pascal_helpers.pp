unit u;
interface
function rotl(d : dword; b : byte) : dword;
function widen(v : shortint; n : byte) : longint;
implementation
function rotl(d : dword; b : byte) : dword;
begin
  rotl := (d shr (32-b)) or (d shl b);
end;
function widen(v : shortint; n : byte) : longint;
begin
  widen := v shl n;
end;
end.
