program p;
var i : integer;
begin
  repeat i := i + 1 until i > 5;
  for i := 1 to 10 do i := i + 1;
  for i := 10 downto 1 do i := i - 1;
  for i in [1, 3, 5] do i := i + 1;
  case i of
    1 : i := 0;
    2, 3 : i := 1;
    4..6 : i := 2;
  else
    i := 99;
  end;
  with i do begin end;
end.
