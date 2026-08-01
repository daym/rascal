unit u;
interface
type
  twoword = record
    lo : longword;
    hi : longword;
  end;
function run(x : double) : longword;
implementation
function run(x : double) : longword;
begin
  run := twoword(x).hi;
end;
end.
