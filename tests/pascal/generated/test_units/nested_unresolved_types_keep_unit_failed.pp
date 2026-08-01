unit main;
interface
type
  trecord = record field : tmissingfield; end;
  tarray = array[0..1] of tmissingelement;
procedure run(value : tmissingparam);
implementation
procedure run(value : tmissingparam);
begin
end;
end.
