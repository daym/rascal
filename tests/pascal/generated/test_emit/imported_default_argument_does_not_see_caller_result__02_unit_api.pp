unit api;
interface
type
  tflag = (enabled);
  tflags = set of tflag;
const
  marker = [enabled];
procedure take(v : tflags = marker);
implementation
procedure take(v : tflags);
begin
end;
end.
