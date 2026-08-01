unit u;
interface
type
  tindex = type word;
  tarr = array[0..2] of byte;
const
  maxindex = high(tindex);
procedure take(a : tindex; const arr : tarr);
implementation
procedure take(a : tindex; const arr : tarr);
begin
  if a = high(a) then begin end;
  while low(arr) <= high(arr) do break;
end;
end.
