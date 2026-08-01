unit u;
interface
function run(x : double) : longword;
implementation
function run(x : double) : longword;
type
  twoword = record
    lo : longword;
    hi : longword;
  end;
begin
  run := twoword(x).hi;
end;
end.
