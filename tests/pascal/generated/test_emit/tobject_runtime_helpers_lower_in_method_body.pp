unit u;
interface
type
  titem = class
    function metaptr : pointer;
    function bytes : integer;
  end;
implementation
function titem.metaptr : pointer;
begin
  metaptr := classtype;
end;
function titem.bytes : integer;
begin
  bytes := instancesize;
end;
end.
