unit u;
interface
type
  tconstexprint = record
    case signed : boolean of
      false : (uvalue : qword);
      true : (svalue : int64);
  end;
  tconstvalue = record
    case tag : longint of
      0 : (valueord : tconstexprint);
      1 : (valueptr : pointer);
  end;
procedure run(var value : tconstvalue; v : qword; var outv : qword);
implementation
procedure run(var value : tconstvalue; v : qword; var outv : qword);
begin
  with value.valueord do begin
    signed := false;
    uvalue := v;
    outv := uvalue;
  end;
end;
end.
