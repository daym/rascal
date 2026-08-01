unit u;
interface
type
  pcaselabel = ^tcaselabel;
  tcaselabel = record
    less : pcaselabel;
    greater : pcaselabel;
  end;
  tlabelarrayentry = record
    caselabel : pcaselabel;
  end;
  tlabelarray = array of tlabelarrayentry;
  tcgcasenode = class
    procedure genjmptree(root : pcaselabel);
  end;
implementation
procedure tcgcasenode.genjmptree(root : pcaselabel);
var
  labelarray : tlabelarray;
procedure rebuild(first,last : int64; var p : pcaselabel);
begin
  p := labelarray[first].caselabel;
end;
begin
  setlength(labelarray,1);
  rebuild(0,high(labelarray),root);
end;
end.
