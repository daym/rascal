unit foo;
interface
implementation
initialization
  x := 1;
finalization
  while x < 3 do begin
    x := x + 1;
  end;
end.
