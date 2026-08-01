unit u;
interface
type
  tflag = (first, second);
  tflags = set of tflag;
function combine : tflags;
implementation
function left : tflags;
begin left := [first]; end;
function right : tflags;
begin right := [second]; end;
function combine : tflags;
begin combine := left() >< right(); end;
end.
