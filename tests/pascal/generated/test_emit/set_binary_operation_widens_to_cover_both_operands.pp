unit u;
interface
type
  tflag = (first, second, third, fourth);
  tflags = set of tflag;
function combine : tflags;
implementation
function combine : tflags;
begin
  combine := ([first] + [fourth]) - [second];
end;
end.
