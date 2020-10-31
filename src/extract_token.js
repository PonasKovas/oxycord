let setRequestHeader = XMLHttpRequest.prototype.setRequestHeader

XMLHttpRequest.prototype.setRequestHeader = function() {
	if (arguments[0] == "Authorization" && arguments[1] && !arguments[1].startsWith("Bearer")) {
		external.invoke(String(arguments[1]))
	}
	setRequestHeader.apply(this, arguments)
}