unit u;
interface
type
  tintbits = record
    overflow : boolean;
    case signed : boolean of
      false : (uvalue : qword);
      true : (svalue : int64);
  end;
function readu(var r : tintbits) : qword;
procedure writeu(var r : tintbits; v : qword);
implementation
function readu(var r : tintbits) : qword;
begin
  readu := r.uvalue;
end;
procedure writeu(var r : tintbits; v : qword);
begin
  r.uvalue := v;
end;
end.
